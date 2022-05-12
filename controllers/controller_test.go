package controllers

import (
	"context"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	"github.com/brianvoe/gofakeit/v6"
	. "github.com/onsi/ginkgo"
	. "github.com/onsi/gomega"
	"github.com/onsi/gomega/gstruct"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
)

// TODO: test different namespaces.
var testNamespace = "default"

func genTask(name string, namespace string) amev1alpha1.Task {
	return amev1alpha1.Task{
		ObjectMeta: v1.ObjectMeta{Name: name, Namespace: namespace},
		TypeMeta:   v1.TypeMeta{APIVersion: "ame.teainspace.com/v1alpha1", Kind: "Task"},
		Spec: amev1alpha1.TaskSpec{
			RunCommand: "python train.py",
			ProjectId:  "myprojectid",
		},
	}
}

func getParameterByName(parameters []argo.Parameter, name string) argo.Parameter {
	for _, p := range parameters {
		if p.Name == name {
			return p
		}
	}

	return argo.Parameter{}
}

var _ = Describe("Task execution", func() {
	AfterEach(func() {
		err := k8sClient.DeleteAllOf(ctx, &argo.Workflow{}, &client.DeleteAllOfOptions{ListOptions: client.ListOptions{Namespace: testNamespace}})
		Expect(err).ToNot(HaveOccurred())

		err = k8sClient.DeleteAllOf(ctx, &amev1alpha1.Task{}, &client.DeleteAllOfOptions{ListOptions: client.ListOptions{Namespace: testNamespace}})
		Expect(err).ToNot(HaveOccurred())
	})

	It("Can create an argo workflow to execute a task", func() {
		// TODO: should we context.Background?
		ctx := context.Background()

		test := genTask(gofakeit.Noun(), testNamespace)

		// Ensure that a Workflow for the Task does not already exist
		// before creating it.
		err := getArgoWorkflow(ctx, k8sClient, test, &argo.Workflow{})
		Expect(err).To(MatchError(newWorkflowNotFoundError(test)))

		err = k8sClient.Create(ctx, &test)
		Expect(err).ToNot(HaveOccurred())

		Eventually(func() (argo.Workflow, error) {
			wf := argo.Workflow{}
			err := getArgoWorkflow(ctx, k8sClient, test, &wf)
			return wf, err
		}, "500ms").Should(gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
			"ObjectMeta": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
				"Namespace": Equal(test.Namespace),
				"Name":      ContainSubstring(test.GetName()),
			}),
		}))
	})

	It("Does recreate an argo workflow if the existing workflow is deleted", func() {
		ctx := context.Background()
		test := genTask(gofakeit.Noun(), testNamespace)

		err := k8sClient.Create(ctx, &test)
		Expect(err).ToNot(HaveOccurred())

		// Ensure that the workflow exists before deleting it.
		expectedWorkflow := argo.Workflow{}
		Eventually(func() error {
			err = getArgoWorkflow(ctx, k8sClient, test, &expectedWorkflow)
			return err
		}, "100ms").Should(Not(HaveOccurred()))

		err = k8sClient.Delete(ctx, &expectedWorkflow)
		Expect(err).ToNot(HaveOccurred())

		// Before verfiying that the UIDs are not equal, it is important to
		// check that the initial Workflow's UID is not empty. As that would
		// make the comparison meaningless.
		Expect(expectedWorkflow.UID).ToNot(BeEmpty())
		Eventually(func() (argo.Workflow, error) {
			wf := argo.Workflow{}
			err = getArgoWorkflow(ctx, k8sClient, test, &wf)
			return wf, err
		}, "1s").Should(gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
			"ObjectMeta": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
				"UID": Not(Equal(expectedWorkflow.UID)),
			}),
		}))
	})

	It("Should reconfigure a Workflow's parameters if they have been misconfigured", func() {
		ctx := context.Background()
		test := genTask(gofakeit.Noun(), testNamespace)
		test.Spec.ProjectId = gofakeit.UUID()

		correctParams := genParameters(test.Spec)

		err := k8sClient.Create(ctx, &test)
		Expect(err).ToNot(HaveOccurred())

		// Cover every potential misconfiguration of the Workflow parameters.
		// TODO when replacing ginkgo with go test we should reimplemented this test as a table driven test.
		testCases := [][]argo.Parameter{
			// Tests that a parameter with an incorrect name and value is removed and the missing requird parameter(s) are put in the parameter list.
			{
				{
					Name:  "wrong-name",
					Value: argo.AnyStringPtr("wrong-value"),
				},
				getParameterByName(correctParams, "run-command"),
			},

			// Tests that a parameter wrong value is replaced with the correct value.
			{
				{
					Name:  "run-command",
					Value: argo.AnyStringPtr("wrong-value"),
				},
				getParameterByName(correctParams, "project-id"),
			},

			// Tests that parameters with an incorrect name but a correct value are replaced.
			{
				getParameterByName(correctParams, "run-command"),
				{
					Name:  "wrong-name2",
					Value: argo.AnyStringPtr(getParameterByName(correctParams, "project-id").Value),
				},
			},

			// Tests that excess parameters are removed, when all reqired parameters are present.
			{
				{
					Name:  "wrong-name",
					Value: argo.AnyStringPtr("wrong-value"),
				},
				correctParams[0],
				correctParams[1],
			},

			// Tests that missing correct parameters are put back in the parameter list.
			{
				correctParams[1],
			},
		}

		// Ensure that we are testing with filled out parameters.
		for _, p := range correctParams {
			Expect(p.Value.String()).ToNot(BeEmpty())
		}

		for _, testCase := range testCases {
			// Ensure that the workflow configuration is correct before changing it.
			wf := argo.Workflow{}
			Eventually(func() (argo.Workflow, error) {
				err := getArgoWorkflow(ctx, k8sClient, test, &wf)
				return wf, err
			}).Should(gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
				"Spec": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
					"Arguments": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
						"Parameters": Equal(correctParams),
					}),
				}),
			}))

			// Change the Workflow specification so it uses the test case parameters.
			wf.Spec.Arguments.Parameters = testCase

			err = k8sClient.Update(ctx, &wf)
			Expect(err).ToNot(HaveOccurred())

			// Validate that the workflow in the cluster has the intended changes.
			// TODO this validation is not great, as it requires the cluster
			// controller to be slow enough to reconcile so we can catch the mis
			// configured Workflow before it is corrected.
			// If having this extra validation makes sense to keep we need should
			// implement a time independent method of validating the changes to
			// the cluster so we know the test will always work.
			err = k8sClient.Get(ctx, client.ObjectKeyFromObject(&wf), &wf)
			Expect(err).ToNot(HaveOccurred())
			Expect(wf.Spec.Arguments.Parameters).To(Equal(testCase))

			// Ensure the Workflow configuration is corrected.
			Eventually(func() (argo.Workflow, error) {
				err := getArgoWorkflow(ctx, k8sClient, test, &wf)
				return wf, err
			}).Should(gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
				"Spec": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
					"Arguments": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
						"Parameters": Equal(correctParams),
					}),
				}),
			}))
		}
	})

	// TODO: should we test that workflows are not recreated if one already exists for a task?
	// TODO: should we test that workflows are deleted when a task is deleted?
})
