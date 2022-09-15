// Package secrets implements tools for interacting with AME secrets.
//
// The initial implementation is a simple abstraction over Kubernetes Secrets. The intent is to keep the outward interface
// more or less unchanged as the implementation is developed, so the rest of the codebase can start using this package without
// needing major changes as the implementaion is developed.
package secrets

import (
	"context"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	clientgocorev1 "k8s.io/client-go/kubernetes/typed/core/v1"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
)

// secretKey is the key used by all AME secrets stored as K8s secrets.
const secretKey = "secret"

// An AmeSecret represents a secret that can be used by a Task.
// The Name is used to identify the Secret and Value is contents in the form of a string.
type AmeSecret struct {
	Name  string
	Value string
}

// A SecretStore represents the Store of secrets AME uses to write and read to.
// AME curently stores secrets as Kubernetes secrets.
type SecretStore struct {
	// secrets is the interface to the k8s REST API for secrets.
	secrets clientgocorev1.SecretInterface
}

// NewSecretStore creates a new SecreteStore which uses secrets to communicate
// with the Kubernetes API. What cluster and namespace secrets is configured
// to use, is also what the SecretStore will use.
// The New SecretStore is returned.
func NewSecretStore(secrets clientgocorev1.SecretInterface) SecretStore {
	return SecretStore{
		secrets: secrets,
	}
}

// ForceCreate creates the secret and overwrites any existing secret with the same name.
// If an error occurs it is returned immediately aborting the secret creation.
func (s *SecretStore) ForceCreate(ctx context.Context, secret AmeSecret) error {
	err := s.Create(ctx, secret)
	if errors.IsAlreadyExists(err) {
		err := s.Delete(ctx, secret.Name)
		if err != nil {
			return err
		}

		return s.Create(ctx, secret)
	}

	if err != nil {
		return err
	}

	return nil
}

// Create attempts to create the secret. If there already exists a secret with the same name,
// an error will be returned, and the existing secret is not overwritten.
// If an error occurs it is returned.
func (s *SecretStore) Create(ctx context.Context, secret AmeSecret) error {
	k8sSecret := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{
			Name: secret.Name,
		},
		StringData: map[string]string{
			secretKey: secret.Value,
		},
	}
	_, err := s.secrets.Create(ctx, k8sSecret.DeepCopy(), metav1.CreateOptions{})
	return err
}

// Delete attempts to delete a secret identified by the name.
// If an error occurs it is returned.
func (s *SecretStore) Delete(ctx context.Context, name string) error {
	return s.secrets.Delete(ctx, name, metav1.DeleteOptions{})
}

// SecretEnvVarSrc returns an EnvVarSource for the secret identified
// by secretName.
func SecretEnvVarSrc(secretName string) *corev1.EnvVarSource {
	return &corev1.EnvVarSource{
		SecretKeyRef: &corev1.SecretKeySelector{
			Key: secretKey,
			LocalObjectReference: corev1.LocalObjectReference{
				Name: secretName,
			},
		},
	}
}
