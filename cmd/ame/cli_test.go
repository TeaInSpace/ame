package main

// TODO: This file is too large it needs to be split up.

import (
	"bytes"
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path"
	"regexp"
	"strings"
	"testing"
	"time"

	argoWf "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	argo "github.com/argoproj/argo-workflows/v3/pkg/client/clientset/versioned/typed/workflow/v1alpha1"
	"github.com/google/go-cmp/cmp"
	"golang.org/x/sync/errgroup"

	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	"github.com/ActiveState/vt10x"
	"github.com/Netflix/go-expect"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
	"teainspace.com/ame/controllers"
	"teainspace.com/ame/generated/clientset/versioned/typed/ame/v1alpha1"
	"teainspace.com/ame/internal/ameproject"
	"teainspace.com/ame/internal/dirtools"
	"teainspace.com/ame/internal/testcfg"
	testenv "teainspace.com/ame/internal/testenv"
	"teainspace.com/ame/server/storage"

	"teainspace.com/ame/internal/clients"
	"teainspace.com/ame/internal/config"
)

const (
	cliName = "ame"
)

var (
	tasks     v1alpha1.TaskInterface
	workflows argo.WorkflowInterface
	ctx       context.Context
	testCfg   testcfg.TestEnvConfig
)

// rmProjectFile Removes the project file if it is present.
// If a project file is not present, the function does nothing.
// If a project file exists and an error occurs when attempting to remove it,
// that error is considered fatal, and the method will fail the current test, using t.
func rmProjectFile(t *testing.T) {
	err := os.Remove(ameproject.AmeProjectFileName)
	// The project file not existing is not considered an error.
	if err != nil && !os.IsNotExist(err) {
		t.Fatal(err)
	}
}

func waitForWorkflowStatus(ctx context.Context, wfName string, timeout time.Duration, targetPhase argoWf.WorkflowPhase) error {
	deadline := time.Now().Add(timeout)

	var actualPhase argoWf.WorkflowPhase

	for {
		if time.Now().After(deadline) {
			return fmt.Errorf(
				"workflow: %s did not reach the target phase: %s within: %v, ended at phase: %s",
				wfName,
				targetPhase,
				timeout,
				actualPhase,
			)
		}

		wf, err := workflows.Get(ctx, wfName, v1.GetOptions{})
		if err != nil {
			return err
		}

		if wf != nil {
			actualPhase = wf.Status.Phase
		}

		if actualPhase == targetPhase {
			return nil
		}

		time.Sleep(time.Millisecond * 100)
	}
}

func getWorkflowForProject(ctx context.Context, projectId string, timeout time.Duration) (*argoWf.Workflow, error) {
	deadline := time.Now().Add(timeout)
	for {
		if time.Now().After(deadline) {
			return nil, fmt.Errorf("from getWorkflowForProject, failed to get workflow for project: %s, within timeout: %v", projectId, timeout)
		}

		projecTasks, err := ameproject.GetTasksForProject(ctx, tasks, projectId)
		if err != nil {
			return nil, err
		}

		if len(projecTasks) > 1 {
			return nil, fmt.Errorf("from getWorkflowForProject found %v tasks for the project but expected %v", len(projecTasks), 1)
		}

		if len(projecTasks) == 0 {
			time.Sleep(time.Millisecond * 50)
			continue
		}

		wf, err := getArgoWorkflow(ctx, &projecTasks[0])
		if err != nil && time.Now().After(deadline) {
			return nil, err
		}

		if err != nil {
			time.Sleep(time.Millisecond * 50)
			continue
		}

		return wf, nil
	}
}

// TODO merge with GetArgoWorkflow in the controllers package.

// getArgoWorkflow retrieves the workflow owned by the task, if such a workflow exists the out object will be populated
// with that workflow.
func getArgoWorkflow(ctx context.Context, task *amev1alpha1.Task) (*argoWf.Workflow, error) {
	// TODO: Find an alternative method of gettting the workflow for a task, without fetching the entire list and filtering it.
	// TODO: How should we handle the possibility of multiple workflows owned by a single task?
	wfList, err := workflows.List(ctx, v1.ListOptions{})
	if err != nil {
		return nil, err
	}

	for _, wf := range wfList.Items {
		for _, or := range wf.GetOwnerReferences() {
			if or.UID == task.GetUID() {
				return &wf, nil
			}
		}
	}

	return nil, controllers.NewWorkflowNotFoundError(*task)
}

func TestMain(m *testing.M) {
	ctx = context.Background()
	testCfg = testcfg.TestEnv()
	kubeCfg, err := clients.KubeClientFromConfig()
	if err != nil {
		log.Fatal(err)
	}

	workflows = clients.WorkflowsClientFromConfig(kubeCfg, testCfg.Namespace)
	tasks = clients.TasksClientFromConfig(kubeCfg, testCfg.Namespace)

	cmd := exec.Command("go", "build", ".")
	err = cmd.Run()
	if err != nil {
		log.Fatal(err)
	}

	exitCode := m.Run()
	// Ensure that the CLI binary is cleanedup.
	os.Remove(cliName)
	os.Exit(exitCode)
}

// genCliCmd returns a new *exec.Cmd with the path to the
// AME CLI binary built for this test run and with the cmd arguments
// set to cmdArgs.
func genCliCmd(cmdArgs ...string) (*exec.Cmd, error) {
	wd, err := os.Getwd()
	if err != nil {
		return nil, err
	}

	cmd := exec.Command(path.Join(wd, cliName))
	cmd.Args = append([]string{""}, cmdArgs...)

	return cmd, nil
}

// matchBuf looks for regex supplied by the parameter pattern in the buffer buf with the
// duration specified by the timeout parameter.
func matchBuf(buf *bytes.Buffer, pattern string, timeout time.Duration) (bool, error) {
	timer := time.NewTimer(timeout)

	for {
		select {
		// A signal from the timer's channel indicates that the timeout duration has passed
		// and the function has therefore failed to find a match for the pattern.
		case <-timer.C:
			return false, nil
		// 10ms loop delays provide enough time for the buffer to have changed,
		// without having excessive delays between each loop.
		default:
			time.Sleep(time.Millisecond * 10)
		}

		matched, err := regexp.MatchString(pattern, buf.String())
		if err != nil {
			return false, err
		}

		if matched {
			return true, nil
		}
	}
}

func startConsole(c *expect.Console) {
	_, err := c.ExpectEOF()
	if err != nil {
		log.Fatalln(err)
	}
}

type behaviors []struct {
	Input     string
	ExpOutput string
}

func generateCreateTaskBehavior(projectName string, taskName string, command string) behaviors {
	return behaviors{
		{
			Input:     projectName,
			ExpOutput: ".*Project name*.",
		},
		{
			Input:     taskName,
			ExpOutput: ".*Task name*.",
		},
		{
			Input:     command,
			ExpOutput: ".*Command*.",
		},
	}
}

func validateCliBehaviorWithCmd(bs behaviors, cmd *exec.Cmd) (string, error) {
	buf := bytes.Buffer{}
	console, err := virtualConsole(cmd, &buf)
	if err != nil {
		return "", err
	}

	defer console.Close()
	go startConsole(console)

	eGroup := validateConsoleBehavior(bs, &buf, console)

	err = waitForCmd(cmd, eGroup)

	// Note that the output is still returned in the case of the
	// command producing an error. That is because the output my
	// still be useful in the test being run.
	return buf.String(), err
}

func validateCliBehavior(bs behaviors, cmdArgs ...string) (string, error) {
	cmd, err := genCliCmd(cmdArgs...)
	if err != nil {
		return "", err
	}

	return validateCliBehaviorWithCmd(bs, cmd)
}

func validateConsoleBehavior(bs behaviors, buf *bytes.Buffer, c *expect.Console) *errgroup.Group {
	egroup := new(errgroup.Group)
	egroup.Go(func() error {
		for _, b := range bs {
			matched, err := matchBuf(buf, b.ExpOutput, time.Millisecond*100)
			if err != nil {
				return err
			}

			if !matched {
				return fmt.Errorf("dit not find %s in output \n%s", b.ExpOutput, buf.String())
			}

			c.SendLine(b.Input)
		}

		c.SendLine("")

		return nil
	})

	return egroup
}

// virtualConsole Creates an expect.Console which duplicates it's output to buf, configures
// cmd to run within the Console, and returns that Console. The Console acts as a simulated
// TTY allowing for testing TTY applications by writing to Console's stdin and reading from buf.
// As an example the prompts generated by the survey library require a TTY to run and therefore also
// for testing.
func virtualConsole(cmd *exec.Cmd, buf *bytes.Buffer) (*expect.Console, error) {
	c, _, err := vt10x.NewVT10XConsole(expect.WithStdout(buf))
	if err != nil {
		return nil, err
	}

	cmd.Stdin = c.Tty()
	cmd.Stdout = c.Tty()

	return c, nil
}

// TODO: this test has grown far too large, look at how to minimize and split it up.

func TestRun(t *testing.T) {
	rmProjectFile(t)
	defer rmProjectFile(t)

	store, err := testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	err = testenv.LoadCliConfigToEnv(testCfg)
	if err != nil {
		t.Error(err)
	}
	defer config.ClearCliCfgFromEnv()

	files := []storage.ProjectFile{
		{
			Path: "somefile.txt",
			Data: []byte("somecontents"),
		},
	}

	testDir, err := dirtools.MkAndPopulateDirTemp("myproject", files)
	if err != nil {
		t.Error(err)
	}
	// The CLI defaults to using the folder name as the project name.
	// Note that the input to MkAndPopulateDirTemp is not the final
	// directory name but only used as prefix for a random name.
	// Hence why exctracting the directory name is necessary.
	projectName := path.Base(testDir)

	testTask := amev1alpha1.NewTask("python test.py", projectName)

	cliCmd, err := genCliCmd("run", testTask.Spec.RunCommand)
	if err != nil {
		t.Fatal(err)
	}
	cliCmd.Dir = testDir // The CLI expects to be executed from the project directory.

	taskName := "trainmodel"
	bs := behaviors{
		{
			Input:     "Y",
			ExpOutput: ".*setup a project*",
		},
	}

	bs = append(bs, generateCreateTaskBehavior(projectName, taskName, testTask.Spec.RunCommand)...)
	out, err := validateCliBehaviorWithCmd(bs, cliCmd)
	if err != nil {
		t.Error(err)
	}
	time.Sleep(time.Second * 1)
	// Validate the specification of the task generated by the CLI.
	taskList, err := tasks.List(ctx, v1.ListOptions{})
	if err != nil {
		t.Fatal(err)
	}

	if len(taskList.Items) != 1 {
		t.Fatalf("expected to find a single task, but found %d instead", len(taskList.Items))
	}

	inclusterTask := taskList.Items[0]

	if testTask.Spec.RunCommand != inclusterTask.Spec.RunCommand {
		t.Errorf("Run created a task with Spec.RunCommand: %s , but the cli received the run command: %s, got the CLI output: %v",
			inclusterTask.Spec.RunCommand,
			testTask.Spec.RunCommand,
			string(out))
	}

	// TODO: this sleep needs to be replaced with propper polling.
	time.Sleep(time.Second * 1)

	// Validate that a Workflow was actually created based on the task.
	wfList, err := workflows.List(ctx, v1.ListOptions{})
	if err != nil {
		t.Error(err)
	}

	if len(wfList.Items) != 1 {
		t.Errorf("Got %d workflows after 1 second, expected %d , \n with CLI output: %s", len(wfList.Items), 1, string(out))
	}

	wf := wfList.Items[0]
	wfRunCmd := controllers.ExtractRunCommand(&wf)
	wfProjectID := controllers.ExtractProjectID(&wf)

	// TODO: We can probably use cmp.Diff to compare the task specifications.
	if testTask.Spec.RunCommand != wfRunCmd {
		t.Errorf("Workflow has run command: %s, but expected: %s, got cli output: %s",
			wfRunCmd,
			testTask.Spec.RunCommand,
			out)
	}

	if testTask.Spec.ProjectId != wfProjectID {
		t.Errorf("Workflow has project ID: %s, but expected: %s",
			wfProjectID,
			testTask.Spec.ProjectId)
	}

	storedFiles, err := store.DownloadFiles(ctx, projectName)
	if err != nil {
		t.Error(err)
	}

	diffs := dirtools.DiffFiles(files, storedFiles)
	if len(diffs) > 0 {
		t.Errorf("The CLI uploaded %+v, expected %+v for project %s, diffs: %v\n stdout: %s", storedFiles, files, projectName, diffs, out)
	}

	// TODO: the code for validating a project file can probably be shared among tests.
	cfg, err := ameproject.ReadProjectFile(cliCmd.Dir)
	if err != nil {
		t.Fatal(err)
	}

	if cfg.ProjectName != projectName {
		t.Errorf("expected the saved project name to be: %s, but got %s", cfg.ProjectName, projectName)
	}

	if len(cfg.Specs) != 1 {
		t.Errorf("expected the project config to only have 1 task, but got %v", len(cfg.Specs))
	}

	diff := cmp.Diff(cfg.Specs[ameproject.TaskSpecName(taskName)], &testTask.Spec)
	if diff != "" {
		t.Errorf("expected the saved task spec to match the run arguments but got diff: %s", diff)
	}
}

func TestCliSetup(t *testing.T) {
	err := config.PrepTmpCfgDir(config.CliConfig{})
	if err != nil {
		t.Error(err)
	}

	cliCmd, err := genCliCmd("setup")
	if err != nil {
		t.Error(err)
	}

	buf := &bytes.Buffer{}
	c, err := virtualConsole(cliCmd, buf)
	if err != nil {
		t.Error(err)
	}

	defer c.Close()
	go startConsole(c)

	correctCfg := config.CliConfig{AuthToken: "mytoken", AmeEndpoint: "https://myendpoint.com"}

	behavior := []struct {
		Input     string
		ExpOutput string
	}{
		{
			Input:     correctCfg.AuthToken,
			ExpOutput: ".*token*.",
		},
		{
			Input:     correctCfg.AmeEndpoint,
			ExpOutput: ".*Endpoint*.",
		},
	}

	go func() {
		for _, b := range behavior {
			matched, err := matchBuf(buf, b.ExpOutput, time.Millisecond*100)
			if err != nil {
				t.Error(err)
			}

			if !matched {
				t.Errorf("buf.String()=%s, expected output to to match regex %s", buf.String(), b.ExpOutput)
			}

			c.SendLine(b.Input)
		}

		c.SendLine("")
	}()

	err = cliCmd.Run()
	if err != nil {
		t.Error(err)
	}

	time.Sleep(time.Second * 2)

	cfg, err := config.GenCliConfig()
	if err != nil {
		t.Errorf("Got error from config generation %v, with cli output: \n%s", err, buf.String())
	}

	diff := cmp.Diff(cfg, correctCfg)
	if diff != "" {
		t.Errorf("Expected %+v == %+v, but got diff: %s", cfg, correctCfg, diff)
	}
}

// TODO: test that the CLI handles errors in task runs.

func TestCanRunPipenvBasedProject(t *testing.T) {
	err := testenv.LoadCliConfigToEnv(testCfg)
	if err != nil {
		t.Fatal(err)
	}
	defer config.ClearCliCfgFromEnv()

	type runBehaviour struct {
		command        string
		expectWfStatus argoWf.WorkflowPhase
		name           string
	}

	tcs := []runBehaviour{
		{
			name:           "can run pipenv based project",
			command:        "python echo.py echo",
			expectWfStatus: argoWf.WorkflowSucceeded,
		},
		{
			name:           "can handle task failure",
			command:        "python sddsf.py echo",
			expectWfStatus: argoWf.WorkflowFailed,
		},
	}

	for _, tc := range tcs {
		_, err := testenv.SetupCluster(ctx, testCfg)
		if err != nil {
			t.Fatal(err)
		}

		t.Run(tc.name, func(t *testing.T) {
			cliCmd, err := genCliCmd("run", tc.command)
			if err != nil {
				t.Fatal(err)
			}

			wd, err := os.Getwd()
			if err != nil {
				t.Fatal(err)
			}
			cliCmd.Dir = path.Join(wd, "../../test_data/test_projects/echo/")

			output, err := cliCmd.CombinedOutput()
			if err != nil {
				t.Fatalf("Got error from cli execution %v, with output: \n%s", err, output)
			}

			if tc.expectWfStatus != argoWf.WorkflowFailed {
				// TODO: find a better way of detecting errors in task runs using the CLI
				if strings.Contains(string(output), "Error") {
					t.Errorf("Got error in CLI output: %s", string(output))
				}
			}

			projectId := "echo"

			// The low timeout is delibarate here, as the workflow should be created immediately.
			wf, err := getWorkflowForProject(ctx, projectId, time.Second)
			if err != nil {
				t.Fatal(err)
			}

			err = waitForWorkflowStatus(ctx, wf.GetName(), time.Second*30, tc.expectWfStatus)
			if err != nil {
				t.Errorf("while waiting for workflow status got error: %v, with logs: %s", err, string(output))
			}
		})
	}
}

// TODO: What else can we validate for a failing task?
func TestCanRunHandleTaskFailure(t *testing.T) {
	_, err := testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	err = testenv.LoadCliConfigToEnv(testCfg)
	if err != nil {
		t.Fatal(err)
	}
	defer config.ClearCliCfgFromEnv()

	cliCmd, err := genCliCmd("run", "python sdfd.py echo")
	if err != nil {
		t.Fatal(err)
	}

	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	cliCmd.Dir = path.Join(wd, "../../test_data/test_projects/echo/")

	output, err := cliCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("Got error from cli execution %v, with output: \n%s", err, output)
	}

	projectId := "echo"
	timeout := time.Second * 30

	wf, err := getWorkflowForProject(ctx, projectId, time.Second)
	if err != nil {
		t.Fatal(err)
	}

	err = waitForWorkflowStatus(ctx, wf.GetName(), timeout, argoWf.WorkflowFailed)
	if err != nil {
		t.Error(err)
	}
}

// TODO: check that artifacts are still synced if the task fails.
func TestCanDownloadArtifacts(t *testing.T) {
	err := testenv.LoadCliConfigToEnv(testCfg)
	if err != nil {
		t.Fatal(err)
	}
	defer config.ClearCliCfgFromEnv()

	_, err = testenv.SetupCluster(ctx, testCfg)
	if err != nil {
		t.Fatal(err)
	}

	expectedContents := "artifactContents"
	cliCmd, err := genCliCmd("run", "python artifact.py "+expectedContents)
	if err != nil {
		t.Fatal(err)
	}

	wd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}

	pathToProject := path.Join(wd, "../../test_data/test_projects/artifacts/")
	cliCmd.Dir = path.Join(pathToProject)

	artifactPath := "generated/myartifact.txt"
	err = os.Remove(path.Join(pathToProject, artifactPath))
	if err != nil && !os.IsNotExist(err) {
		t.Fatal(err)
	}

	originalDirSnapshot, err := dirtools.SnapDir(pathToProject)
	if err != nil {
		t.Fatal(err)
	}

	output, err := cliCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("Got error from cli execution %v, with output: \n%s", err, output)
	}

	projectId := "artifacts"
	timeout := time.Second * 240
	wf, err := getWorkflowForProject(ctx, projectId, time.Second)
	if err != nil {
		t.Fatal(err)
	}

	err = waitForWorkflowStatus(ctx, wf.GetName(), timeout, argoWf.WorkflowSucceeded)
	if err != nil {
		wfs, wferr := workflows.List(ctx, v1.ListOptions{})
		if wferr != nil {
			t.Error(err)
		}
		t.Fatalf("got error while waiting for workflow to succeed: %v, with output: %s, wfs: %+v", err, string(output), wfs)
	}

	dirSnapshot, err := dirtools.SnapDir(pathToProject)
	if err != nil {
		t.Fatal(err)
	}

	snapDiff := dirtools.SnapshotDiff(originalDirSnapshot, dirSnapshot)

	if len(snapDiff) != 1 {
		t.Errorf("Should only have found one new file, but got diff: %+v", snapDiff)
	}

	if snapDiff[0].RelativePath != artifactPath {
		t.Errorf("expected new file to be at the artifact path: %s, but was at path: %s", artifactPath, snapDiff[0].RelativePath)
	}

	contents, err := os.ReadFile(path.Join(pathToProject, "generated/myartifact.txt"))
	if err != nil {
		t.Fatal(err)
	}

	diff := cmp.Diff(string(contents), expectedContents)
	if diff != "" {
		t.Errorf("expected artifact contents %s does not equal actual contents %s, diff: %s ", expectedContents, string(contents), diff)
	}

	fmt.Println(string(output))
}

func TestCreateTaskConfig(t *testing.T) {
	rmProjectFile(t)
	defer rmProjectFile(t)

	taskName := "mytask"
	correctProjectFileCfg := &ameproject.ProjectFileCfg{
		ProjectName: "myproject",
		Specs: ameproject.TaskSpecs{
			ameproject.TaskSpecName(taskName): &amev1alpha1.TaskSpec{
				RunCommand: "python train.py",
				ProjectId:  "myproject",
			},
		},
	}

	bs := generateCreateTaskBehavior(correctProjectFileCfg.ProjectName, taskName, correctProjectFileCfg.Specs[ameproject.TaskSpecName(taskName)].RunCommand)

	_, err := validateCliBehavior(bs, "create")
	if err != nil {
		t.Error(err)
	}

	time.Sleep(time.Second * 2)

	projectFileCfg, err := ameproject.ReadProjectFile(".")
	if err != nil {
		t.Fatal(err)
	}

	diff := cmp.Diff(projectFileCfg, correctProjectFileCfg)
	if diff != "" {
		t.Errorf("expected diff between project file configs to be empty but got diffL %s", diff)
	}
}

func waitForCmd(cmd *exec.Cmd, egroup *errgroup.Group) error {
	// It is important that the command is started in a none blocking
	// fashion so we can exit if the error group encounters an error while the command
	// is running.
	// This avoids the command waiting until the test times if something goes wrong and it
	// doesn't get any input.
	err := cmd.Start()
	if err != nil {
		return err
	}

	// This ensures that cmd is always cleaned up by wait.
	defer func() {
		err := cmd.Wait()
		if err != nil {
			fmt.Println(err)
		}
	}()

	err = egroup.Wait()
	if err != nil {
		return err
	}

	return nil
}

func TestCanAddTaskToExistingConfig(t *testing.T) {
	rmProjectFile(t)
	defer rmProjectFile(t)

	taska := amev1alpha1.NewTask("python train.py", "myproject")
	taskb := amev1alpha1.NewTask("python preprocess.py", "myproject")

	cfg := ameproject.NewProjectFileBuilder().AddTaskSpecs(ameproject.TaskSpecs{"taska": &taska.Spec}).SetProjectName(taska.Spec.ProjectId).Build()
	err := ameproject.WriteToProjectFile(".", cfg)
	if err != nil {
		t.Fatal(err)
	}

	taskbName := "taskb"
	bs := behaviors{
		{
			Input:     taskbName,
			ExpOutput: ".*Task name*.",
		},
		{
			Input:     taskb.Spec.RunCommand,
			ExpOutput: ".*Command*.",
		},
	}

	_, err = validateCliBehavior(bs, "create")
	if err != nil {
		t.Error(err)
	}

	newCfg, err := ameproject.ReadProjectFile(".")
	if err != nil {
		t.Fatal(err)
	}

	if newCfg.ProjectName != cfg.ProjectName {
		t.Errorf("expected project name to not have been altered, but it was changed from %s to %s", cfg.ProjectName, newCfg.ProjectName)
	}

	if len(newCfg.Specs) != 2 {
		t.Errorf("expected len(newCfg.Specs)=2, but got %v instead", len(newCfg.Specs))
	}

	taskAName := "taska"
	taskADiff := cmp.Diff(cfg.Specs[ameproject.TaskSpecName(taskAName)], newCfg.Specs[ameproject.TaskSpecName(taskAName)])
	if taskADiff != "" {
		t.Errorf("expected task: %s to not have changed, but got diff: %s", taskAName, taskADiff)
	}

	taskBDiff := cmp.Diff(newCfg.Specs[ameproject.TaskSpecName(taskbName)], &taskb.Spec)

	if taskBDiff != "" {
		t.Errorf("expected the saved spec for taskb to be correct, but got diff: %s", taskBDiff)
	}
}
