package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	runtime "k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
)

func GenOwnRef(obj metav1.ObjectMeta, gvks []schema.GroupVersionKind) metav1.OwnerReference {
	// TODO: Deal with case of len(gvks) == 0 and len(gvks)>1
	return metav1.OwnerReference{
		APIVersion: gvks[0].GroupVersion().String(), // TODO: is there a better method for getting the APIVersion string?
		Kind:       gvks[0].Kind,
		Name:       obj.GetName(),
		UID:        obj.GetUID(),
	}
}

func GenGvks(scheme *runtime.Scheme, obj runtime.Object) ([]schema.GroupVersionKind, error) {
	gvks, _, err := scheme.ObjectKinds(obj)
	if err != nil {
		return nil, err
	}

	return gvks, nil
}
