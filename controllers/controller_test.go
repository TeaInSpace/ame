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
	}
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

	// TODO: should we test that workflows are not recreated if one already exists for a task?
	// TODO: should we test that workflows are deleted when a task is deleted?
})
