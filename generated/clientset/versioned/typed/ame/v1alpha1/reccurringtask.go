/*
Copyright 2022.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

// Code generated by client-gen. DO NOT EDIT.

package v1alpha1

import (
	"context"
	scheme "teainspace.com/ame/generated/clientset/versioned/scheme"
	"time"

	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	types "k8s.io/apimachinery/pkg/types"
	watch "k8s.io/apimachinery/pkg/watch"
	rest "k8s.io/client-go/rest"
	v1alpha1 "teainspace.com/ame/api/v1alpha1"
)

// ReccurringTasksGetter has a method to return a ReccurringTaskInterface.
// A group's client should implement this interface.
type ReccurringTasksGetter interface {
	ReccurringTasks(namespace string) ReccurringTaskInterface
}

// ReccurringTaskInterface has methods to work with ReccurringTask resources.
type ReccurringTaskInterface interface {
	Create(ctx context.Context, reccurringTask *v1alpha1.ReccurringTask, opts v1.CreateOptions) (*v1alpha1.ReccurringTask, error)
	Update(ctx context.Context, reccurringTask *v1alpha1.ReccurringTask, opts v1.UpdateOptions) (*v1alpha1.ReccurringTask, error)
	UpdateStatus(ctx context.Context, reccurringTask *v1alpha1.ReccurringTask, opts v1.UpdateOptions) (*v1alpha1.ReccurringTask, error)
	Delete(ctx context.Context, name string, opts v1.DeleteOptions) error
	DeleteCollection(ctx context.Context, opts v1.DeleteOptions, listOpts v1.ListOptions) error
	Get(ctx context.Context, name string, opts v1.GetOptions) (*v1alpha1.ReccurringTask, error)
	List(ctx context.Context, opts v1.ListOptions) (*v1alpha1.ReccurringTaskList, error)
	Watch(ctx context.Context, opts v1.ListOptions) (watch.Interface, error)
	Patch(ctx context.Context, name string, pt types.PatchType, data []byte, opts v1.PatchOptions, subresources ...string) (result *v1alpha1.ReccurringTask, err error)
	ReccurringTaskExpansion
}

// reccurringTasks implements ReccurringTaskInterface
type reccurringTasks struct {
	client rest.Interface
	ns     string
}

// newReccurringTasks returns a ReccurringTasks
func newReccurringTasks(c *AmeV1alpha1Client, namespace string) *reccurringTasks {
	return &reccurringTasks{
		client: c.RESTClient(),
		ns:     namespace,
	}
}

// Get takes name of the reccurringTask, and returns the corresponding reccurringTask object, and an error if there is any.
func (c *reccurringTasks) Get(ctx context.Context, name string, options v1.GetOptions) (result *v1alpha1.ReccurringTask, err error) {
	result = &v1alpha1.ReccurringTask{}
	err = c.client.Get().
		Namespace(c.ns).
		Resource("reccurringtasks").
		Name(name).
		VersionedParams(&options, scheme.ParameterCodec).
		Do(ctx).
		Into(result)
	return
}

// List takes label and field selectors, and returns the list of ReccurringTasks that match those selectors.
func (c *reccurringTasks) List(ctx context.Context, opts v1.ListOptions) (result *v1alpha1.ReccurringTaskList, err error) {
	var timeout time.Duration
	if opts.TimeoutSeconds != nil {
		timeout = time.Duration(*opts.TimeoutSeconds) * time.Second
	}
	result = &v1alpha1.ReccurringTaskList{}
	err = c.client.Get().
		Namespace(c.ns).
		Resource("reccurringtasks").
		VersionedParams(&opts, scheme.ParameterCodec).
		Timeout(timeout).
		Do(ctx).
		Into(result)
	return
}

// Watch returns a watch.Interface that watches the requested reccurringTasks.
func (c *reccurringTasks) Watch(ctx context.Context, opts v1.ListOptions) (watch.Interface, error) {
	var timeout time.Duration
	if opts.TimeoutSeconds != nil {
		timeout = time.Duration(*opts.TimeoutSeconds) * time.Second
	}
	opts.Watch = true
	return c.client.Get().
		Namespace(c.ns).
		Resource("reccurringtasks").
		VersionedParams(&opts, scheme.ParameterCodec).
		Timeout(timeout).
		Watch(ctx)
}

// Create takes the representation of a reccurringTask and creates it.  Returns the server's representation of the reccurringTask, and an error, if there is any.
func (c *reccurringTasks) Create(ctx context.Context, reccurringTask *v1alpha1.ReccurringTask, opts v1.CreateOptions) (result *v1alpha1.ReccurringTask, err error) {
	result = &v1alpha1.ReccurringTask{}
	err = c.client.Post().
		Namespace(c.ns).
		Resource("reccurringtasks").
		VersionedParams(&opts, scheme.ParameterCodec).
		Body(reccurringTask).
		Do(ctx).
		Into(result)
	return
}

// Update takes the representation of a reccurringTask and updates it. Returns the server's representation of the reccurringTask, and an error, if there is any.
func (c *reccurringTasks) Update(ctx context.Context, reccurringTask *v1alpha1.ReccurringTask, opts v1.UpdateOptions) (result *v1alpha1.ReccurringTask, err error) {
	result = &v1alpha1.ReccurringTask{}
	err = c.client.Put().
		Namespace(c.ns).
		Resource("reccurringtasks").
		Name(reccurringTask.Name).
		VersionedParams(&opts, scheme.ParameterCodec).
		Body(reccurringTask).
		Do(ctx).
		Into(result)
	return
}

// UpdateStatus was generated because the type contains a Status member.
// Add a +genclient:noStatus comment above the type to avoid generating UpdateStatus().
func (c *reccurringTasks) UpdateStatus(ctx context.Context, reccurringTask *v1alpha1.ReccurringTask, opts v1.UpdateOptions) (result *v1alpha1.ReccurringTask, err error) {
	result = &v1alpha1.ReccurringTask{}
	err = c.client.Put().
		Namespace(c.ns).
		Resource("reccurringtasks").
		Name(reccurringTask.Name).
		SubResource("status").
		VersionedParams(&opts, scheme.ParameterCodec).
		Body(reccurringTask).
		Do(ctx).
		Into(result)
	return
}

// Delete takes name of the reccurringTask and deletes it. Returns an error if one occurs.
func (c *reccurringTasks) Delete(ctx context.Context, name string, opts v1.DeleteOptions) error {
	return c.client.Delete().
		Namespace(c.ns).
		Resource("reccurringtasks").
		Name(name).
		Body(&opts).
		Do(ctx).
		Error()
}

// DeleteCollection deletes a collection of objects.
func (c *reccurringTasks) DeleteCollection(ctx context.Context, opts v1.DeleteOptions, listOpts v1.ListOptions) error {
	var timeout time.Duration
	if listOpts.TimeoutSeconds != nil {
		timeout = time.Duration(*listOpts.TimeoutSeconds) * time.Second
	}
	return c.client.Delete().
		Namespace(c.ns).
		Resource("reccurringtasks").
		VersionedParams(&listOpts, scheme.ParameterCodec).
		Timeout(timeout).
		Body(&opts).
		Do(ctx).
		Error()
}

// Patch applies the patch and returns the patched reccurringTask.
func (c *reccurringTasks) Patch(ctx context.Context, name string, pt types.PatchType, data []byte, opts v1.PatchOptions, subresources ...string) (result *v1alpha1.ReccurringTask, err error) {
	result = &v1alpha1.ReccurringTask{}
	err = c.client.Patch(pt).
		Namespace(c.ns).
		Resource("reccurringtasks").
		Name(name).
		SubResource(subresources...).
		VersionedParams(&opts, scheme.ParameterCodec).
		Body(data).
		Do(ctx).
		Into(result)
	return
}
