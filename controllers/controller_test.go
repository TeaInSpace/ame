package controllers

import (
	"context"
	"fmt"

	argo "github.com/argoproj/argo-workflows/v3/pkg/apis/workflow/v1alpha1"
	. "github.com/onsi/ginkgo"
	. "github.com/onsi/gomega"
	"github.com/onsi/gomega/gstruct"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"
	amev1alpha1 "teainspace.com/ame/api/v1alpha1"
)

var _ = Describe("Task execution", func() {
	It("Can create an argo workflow to execute a task", func() {
		ctx := context.Background()
		test := amev1alpha1.Task{ObjectMeta: v1.ObjectMeta{Name: "test", Namespace: "default"}}

		// Ensure that the workflow does not already exist.
		err := k8sClient.Get(ctx, client.ObjectKey{
			Namespace: test.Namespace,
			Name:      test.Name,
		}, &argo.Workflow{})
		Expect(err).To(MatchError(fmt.Sprintf("workflows.argoproj.io \"%s\" not found", test.Name)))

		err = k8sClient.Create(ctx, &test)
		Expect(err).ToNot(HaveOccurred())

		Eventually(func() (argo.Workflow, error) {
			wf := argo.Workflow{}
			err := k8sClient.Get(ctx, client.ObjectKey{Namespace: test.Namespace, Name: test.Name}, &wf)
			return wf, err
		}, "100ms").Should(gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
			"ObjectMeta": gstruct.MatchFields(gstruct.IgnoreExtras, gstruct.Fields{
				"Name":      Equal(test.Name),
				"Namespace": Equal(test.Namespace),
			}),
		}))
	})
})
