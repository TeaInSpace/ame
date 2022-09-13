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
// Code generated by protoc-gen-gogo. DO NOT EDIT.
// source: teainspace.com/ame/pkg/apis/ame/v1alpha1/generated.proto

package v1alpha1

import (
	fmt "fmt"

	io "io"
	math "math"
	math_bits "math/bits"
	reflect "reflect"
	strings "strings"

	proto "github.com/gogo/protobuf/proto"
)

// Reference imports to suppress errors if they are not otherwise used.
var _ = proto.Marshal
var _ = fmt.Errorf
var _ = math.Inf

// This is a compile-time assertion to ensure that this generated file
// is compatible with the proto package it is being compiled against.
// A compilation error at this line likely means your copy of the
// proto package needs to be updated.
const _ = proto.GoGoProtoPackageIsVersion3 // please upgrade the proto package

func (m *Task) Reset()      { *m = Task{} }
func (*Task) ProtoMessage() {}
func (*Task) Descriptor() ([]byte, []int) {
	return fileDescriptor_82a756a2a92d3306, []int{0}
}
func (m *Task) XXX_Unmarshal(b []byte) error {
	return m.Unmarshal(b)
}
func (m *Task) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	b = b[:cap(b)]
	n, err := m.MarshalToSizedBuffer(b)
	if err != nil {
		return nil, err
	}
	return b[:n], nil
}
func (m *Task) XXX_Merge(src proto.Message) {
	xxx_messageInfo_Task.Merge(m, src)
}
func (m *Task) XXX_Size() int {
	return m.Size()
}
func (m *Task) XXX_DiscardUnknown() {
	xxx_messageInfo_Task.DiscardUnknown(m)
}

var xxx_messageInfo_Task proto.InternalMessageInfo

func (m *TaskEnvVar) Reset()      { *m = TaskEnvVar{} }
func (*TaskEnvVar) ProtoMessage() {}
func (*TaskEnvVar) Descriptor() ([]byte, []int) {
	return fileDescriptor_82a756a2a92d3306, []int{1}
}
func (m *TaskEnvVar) XXX_Unmarshal(b []byte) error {
	return m.Unmarshal(b)
}
func (m *TaskEnvVar) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	b = b[:cap(b)]
	n, err := m.MarshalToSizedBuffer(b)
	if err != nil {
		return nil, err
	}
	return b[:n], nil
}
func (m *TaskEnvVar) XXX_Merge(src proto.Message) {
	xxx_messageInfo_TaskEnvVar.Merge(m, src)
}
func (m *TaskEnvVar) XXX_Size() int {
	return m.Size()
}
func (m *TaskEnvVar) XXX_DiscardUnknown() {
	xxx_messageInfo_TaskEnvVar.DiscardUnknown(m)
}

var xxx_messageInfo_TaskEnvVar proto.InternalMessageInfo

func (m *TaskList) Reset()      { *m = TaskList{} }
func (*TaskList) ProtoMessage() {}
func (*TaskList) Descriptor() ([]byte, []int) {
	return fileDescriptor_82a756a2a92d3306, []int{2}
}
func (m *TaskList) XXX_Unmarshal(b []byte) error {
	return m.Unmarshal(b)
}
func (m *TaskList) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	b = b[:cap(b)]
	n, err := m.MarshalToSizedBuffer(b)
	if err != nil {
		return nil, err
	}
	return b[:n], nil
}
func (m *TaskList) XXX_Merge(src proto.Message) {
	xxx_messageInfo_TaskList.Merge(m, src)
}
func (m *TaskList) XXX_Size() int {
	return m.Size()
}
func (m *TaskList) XXX_DiscardUnknown() {
	xxx_messageInfo_TaskList.DiscardUnknown(m)
}

var xxx_messageInfo_TaskList proto.InternalMessageInfo

func (m *TaskSpec) Reset()      { *m = TaskSpec{} }
func (*TaskSpec) ProtoMessage() {}
func (*TaskSpec) Descriptor() ([]byte, []int) {
	return fileDescriptor_82a756a2a92d3306, []int{3}
}
func (m *TaskSpec) XXX_Unmarshal(b []byte) error {
	return m.Unmarshal(b)
}
func (m *TaskSpec) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	b = b[:cap(b)]
	n, err := m.MarshalToSizedBuffer(b)
	if err != nil {
		return nil, err
	}
	return b[:n], nil
}
func (m *TaskSpec) XXX_Merge(src proto.Message) {
	xxx_messageInfo_TaskSpec.Merge(m, src)
}
func (m *TaskSpec) XXX_Size() int {
	return m.Size()
}
func (m *TaskSpec) XXX_DiscardUnknown() {
	xxx_messageInfo_TaskSpec.DiscardUnknown(m)
}

var xxx_messageInfo_TaskSpec proto.InternalMessageInfo

func (m *TaskStatus) Reset()      { *m = TaskStatus{} }
func (*TaskStatus) ProtoMessage() {}
func (*TaskStatus) Descriptor() ([]byte, []int) {
	return fileDescriptor_82a756a2a92d3306, []int{4}
}
func (m *TaskStatus) XXX_Unmarshal(b []byte) error {
	return m.Unmarshal(b)
}
func (m *TaskStatus) XXX_Marshal(b []byte, deterministic bool) ([]byte, error) {
	b = b[:cap(b)]
	n, err := m.MarshalToSizedBuffer(b)
	if err != nil {
		return nil, err
	}
	return b[:n], nil
}
func (m *TaskStatus) XXX_Merge(src proto.Message) {
	xxx_messageInfo_TaskStatus.Merge(m, src)
}
func (m *TaskStatus) XXX_Size() int {
	return m.Size()
}
func (m *TaskStatus) XXX_DiscardUnknown() {
	xxx_messageInfo_TaskStatus.DiscardUnknown(m)
}

var xxx_messageInfo_TaskStatus proto.InternalMessageInfo

func init() {
	proto.RegisterType((*Task)(nil), "teainspace.com.ame.pkg.apis.ame.v1alpha1.Task")
	proto.RegisterType((*TaskEnvVar)(nil), "teainspace.com.ame.pkg.apis.ame.v1alpha1.TaskEnvVar")
	proto.RegisterType((*TaskList)(nil), "teainspace.com.ame.pkg.apis.ame.v1alpha1.TaskList")
	proto.RegisterType((*TaskSpec)(nil), "teainspace.com.ame.pkg.apis.ame.v1alpha1.TaskSpec")
	proto.RegisterType((*TaskStatus)(nil), "teainspace.com.ame.pkg.apis.ame.v1alpha1.TaskStatus")
}

func init() {
	proto.RegisterFile("teainspace.com/ame/pkg/apis/ame/v1alpha1/generated.proto", fileDescriptor_82a756a2a92d3306)
}

var fileDescriptor_82a756a2a92d3306 = []byte{
	// 502 bytes of a gzipped FileDescriptorProto
	0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0x94, 0x92, 0xbf, 0x6e, 0xdb, 0x30,
	0x10, 0xc6, 0x25, 0xcb, 0x0e, 0x6c, 0x26, 0x2d, 0x5a, 0x4e, 0x42, 0x06, 0xc5, 0x50, 0x97, 0x2c,
	0xa5, 0x6a, 0x23, 0x43, 0x66, 0x15, 0x19, 0x02, 0xf4, 0x1f, 0xe8, 0x20, 0x43, 0x91, 0xa1, 0x67,
	0x89, 0x95, 0x55, 0x87, 0x94, 0x20, 0x51, 0x02, 0xba, 0xf5, 0x11, 0xfa, 0x00, 0x7d, 0x95, 0x6e,
	0x1d, 0x3c, 0x66, 0xcc, 0x14, 0xd4, 0xea, 0x8b, 0x14, 0xa4, 0xe8, 0xc8, 0x41, 0x96, 0x78, 0xd3,
	0xf1, 0xee, 0xfb, 0xe9, 0xe3, 0x77, 0x44, 0xa7, 0x92, 0x41, 0x2a, 0xca, 0x1c, 0x22, 0x46, 0xa2,
	0x8c, 0x07, 0xc0, 0x59, 0x90, 0x2f, 0x93, 0x00, 0xf2, 0xb4, 0xd4, 0x45, 0x3d, 0x81, 0xeb, 0x7c,
	0x01, 0x93, 0x20, 0x61, 0x82, 0x15, 0x20, 0x59, 0x4c, 0xf2, 0x22, 0x93, 0x19, 0x3e, 0x7e, 0xa8,
	0x24, 0xc0, 0x19, 0xc9, 0x97, 0x09, 0x51, 0x4a, 0x5d, 0x6c, 0x94, 0x87, 0xaf, 0x93, 0x54, 0x2e,
	0xaa, 0xb9, 0xe6, 0x27, 0x59, 0x92, 0x05, 0x1a, 0x30, 0xaf, 0xbe, 0xea, 0x4a, 0x17, 0xfa, 0xab,
	0x05, 0x1f, 0x9e, 0x2c, 0x4f, 0x4b, 0x92, 0x66, 0xca, 0x02, 0x87, 0x68, 0x91, 0x0a, 0x56, 0x7c,
	0xef, 0x3c, 0x71, 0x26, 0x21, 0xa8, 0x1f, 0xd9, 0xf1, 0x7f, 0xf5, 0x50, 0xff, 0x02, 0xca, 0x25,
	0xfe, 0x82, 0x86, 0x6a, 0x26, 0x06, 0x09, 0xae, 0x3d, 0xb6, 0x8f, 0xf7, 0xa7, 0x6f, 0x48, 0x4b,
	0x24, 0xdb, 0xc4, 0xce, 0xab, 0x9a, 0x26, 0xf5, 0x84, 0x7c, 0x9c, 0x7f, 0x63, 0x91, 0x7c, 0xcf,
	0x24, 0x84, 0x78, 0x75, 0x77, 0x64, 0x35, 0x77, 0x47, 0xa8, 0x3b, 0xa3, 0xf7, 0x54, 0x7c, 0x81,
	0xfa, 0x65, 0xce, 0x22, 0xb7, 0xa7, 0xe9, 0x53, 0xf2, 0xd4, 0x20, 0x88, 0xf2, 0x37, 0xcb, 0x59,
	0x14, 0x1e, 0x18, 0x7e, 0x5f, 0x55, 0x54, 0xd3, 0xf0, 0x15, 0xda, 0x2b, 0x25, 0xc8, 0xaa, 0x74,
	0x1d, 0xcd, 0x3d, 0xd9, 0x91, 0xab, 0xb5, 0xe1, 0x73, 0x43, 0xde, 0x6b, 0x6b, 0x6a, 0x98, 0xfe,
	0x0c, 0x21, 0x35, 0x75, 0x26, 0xea, 0x4b, 0x28, 0xf0, 0x18, 0xf5, 0x05, 0x70, 0xa6, 0xf3, 0x19,
	0x75, 0x6e, 0x3e, 0x00, 0x67, 0x54, 0x77, 0xf0, 0x2b, 0x34, 0xa8, 0xe1, 0xba, 0x62, 0xfa, 0x92,
	0xa3, 0xf0, 0x99, 0x19, 0x19, 0x5c, 0xaa, 0x43, 0xda, 0xf6, 0xfc, 0xdf, 0x36, 0x1a, 0x2a, 0xea,
	0xbb, 0xb4, 0x94, 0xf8, 0xea, 0x51, 0xee, 0xe4, 0x69, 0xb9, 0x2b, 0xb5, 0x4e, 0xfd, 0x85, 0xf9,
	0xc9, 0x70, 0x73, 0xb2, 0x95, 0xf9, 0x0c, 0x0d, 0x52, 0xc9, 0x78, 0xe9, 0xf6, 0xc6, 0x8e, 0x46,
	0xef, 0x14, 0x4e, 0xe7, 0xff, 0x5c, 0x41, 0x68, 0xcb, 0xf2, 0xff, 0x18, 0xff, 0x6a, 0x0b, 0x78,
	0x8a, 0x50, 0x51, 0x89, 0x28, 0xe3, 0x1c, 0x44, 0x6c, 0x92, 0xb9, 0x7f, 0x07, 0xb4, 0x12, 0x6f,
	0xdb, 0x0e, 0xdd, 0x9a, 0xc2, 0x01, 0x1a, 0xe5, 0x45, 0xa6, 0x9e, 0x48, 0x1a, 0x9b, 0xa4, 0x5e,
	0x1a, 0xc9, 0xe8, 0x53, 0xdb, 0x38, 0x8f, 0x69, 0x37, 0x83, 0x67, 0xc8, 0x61, 0xa2, 0x76, 0x1d,
	0x7d, 0x89, 0x1d, 0x37, 0xdc, 0xee, 0x2e, 0xdc, 0x37, 0x3f, 0x70, 0xce, 0x44, 0x4d, 0x15, 0xcd,
	0x3f, 0x68, 0x77, 0x6b, 0x5e, 0x00, 0x59, 0xad, 0x3d, 0xeb, 0x66, 0xed, 0x59, 0xb7, 0x6b, 0xcf,
	0xfa, 0xd1, 0x78, 0xf6, 0xaa, 0xf1, 0xec, 0x9b, 0xc6, 0xb3, 0x6f, 0x1b, 0xcf, 0xfe, 0xdb, 0x78,
	0xf6, 0xcf, 0x7f, 0x9e, 0xf5, 0x79, 0xb8, 0x41, 0xff, 0x0f, 0x00, 0x00, 0xff, 0xff, 0x14, 0xc2,
	0x7c, 0x99, 0x02, 0x04, 0x00, 0x00,
}

func (m *Task) Marshal() (dAtA []byte, err error) {
	size := m.Size()
	dAtA = make([]byte, size)
	n, err := m.MarshalToSizedBuffer(dAtA[:size])
	if err != nil {
		return nil, err
	}
	return dAtA[:n], nil
}

func (m *Task) MarshalTo(dAtA []byte) (int, error) {
	size := m.Size()
	return m.MarshalToSizedBuffer(dAtA[:size])
}

func (m *Task) MarshalToSizedBuffer(dAtA []byte) (int, error) {
	i := len(dAtA)
	_ = i
	var l int
	_ = l
	{
		size, err := m.Status.MarshalToSizedBuffer(dAtA[:i])
		if err != nil {
			return 0, err
		}
		i -= size
		i = encodeVarintGenerated(dAtA, i, uint64(size))
	}
	i--
	dAtA[i] = 0x1a
	{
		size, err := m.Spec.MarshalToSizedBuffer(dAtA[:i])
		if err != nil {
			return 0, err
		}
		i -= size
		i = encodeVarintGenerated(dAtA, i, uint64(size))
	}
	i--
	dAtA[i] = 0x12
	{
		size, err := m.ObjectMeta.MarshalToSizedBuffer(dAtA[:i])
		if err != nil {
			return 0, err
		}
		i -= size
		i = encodeVarintGenerated(dAtA, i, uint64(size))
	}
	i--
	dAtA[i] = 0xa
	return len(dAtA) - i, nil
}

func (m *TaskEnvVar) Marshal() (dAtA []byte, err error) {
	size := m.Size()
	dAtA = make([]byte, size)
	n, err := m.MarshalToSizedBuffer(dAtA[:size])
	if err != nil {
		return nil, err
	}
	return dAtA[:n], nil
}

func (m *TaskEnvVar) MarshalTo(dAtA []byte) (int, error) {
	size := m.Size()
	return m.MarshalToSizedBuffer(dAtA[:size])
}

func (m *TaskEnvVar) MarshalToSizedBuffer(dAtA []byte) (int, error) {
	i := len(dAtA)
	_ = i
	var l int
	_ = l
	i -= len(m.Value)
	copy(dAtA[i:], m.Value)
	i = encodeVarintGenerated(dAtA, i, uint64(len(m.Value)))
	i--
	dAtA[i] = 0x12
	i -= len(m.Name)
	copy(dAtA[i:], m.Name)
	i = encodeVarintGenerated(dAtA, i, uint64(len(m.Name)))
	i--
	dAtA[i] = 0xa
	return len(dAtA) - i, nil
}

func (m *TaskList) Marshal() (dAtA []byte, err error) {
	size := m.Size()
	dAtA = make([]byte, size)
	n, err := m.MarshalToSizedBuffer(dAtA[:size])
	if err != nil {
		return nil, err
	}
	return dAtA[:n], nil
}

func (m *TaskList) MarshalTo(dAtA []byte) (int, error) {
	size := m.Size()
	return m.MarshalToSizedBuffer(dAtA[:size])
}

func (m *TaskList) MarshalToSizedBuffer(dAtA []byte) (int, error) {
	i := len(dAtA)
	_ = i
	var l int
	_ = l
	if len(m.Items) > 0 {
		for iNdEx := len(m.Items) - 1; iNdEx >= 0; iNdEx-- {
			{
				size, err := m.Items[iNdEx].MarshalToSizedBuffer(dAtA[:i])
				if err != nil {
					return 0, err
				}
				i -= size
				i = encodeVarintGenerated(dAtA, i, uint64(size))
			}
			i--
			dAtA[i] = 0x12
		}
	}
	{
		size, err := m.ListMeta.MarshalToSizedBuffer(dAtA[:i])
		if err != nil {
			return 0, err
		}
		i -= size
		i = encodeVarintGenerated(dAtA, i, uint64(size))
	}
	i--
	dAtA[i] = 0xa
	return len(dAtA) - i, nil
}

func (m *TaskSpec) Marshal() (dAtA []byte, err error) {
	size := m.Size()
	dAtA = make([]byte, size)
	n, err := m.MarshalToSizedBuffer(dAtA[:size])
	if err != nil {
		return nil, err
	}
	return dAtA[:n], nil
}

func (m *TaskSpec) MarshalTo(dAtA []byte) (int, error) {
	size := m.Size()
	return m.MarshalToSizedBuffer(dAtA[:size])
}

func (m *TaskSpec) MarshalToSizedBuffer(dAtA []byte) (int, error) {
	i := len(dAtA)
	_ = i
	var l int
	_ = l
	if len(m.Env) > 0 {
		for iNdEx := len(m.Env) - 1; iNdEx >= 0; iNdEx-- {
			{
				size, err := m.Env[iNdEx].MarshalToSizedBuffer(dAtA[:i])
				if err != nil {
					return 0, err
				}
				i -= size
				i = encodeVarintGenerated(dAtA, i, uint64(size))
			}
			i--
			dAtA[i] = 0x1a
		}
	}
	i -= len(m.ProjectId)
	copy(dAtA[i:], m.ProjectId)
	i = encodeVarintGenerated(dAtA, i, uint64(len(m.ProjectId)))
	i--
	dAtA[i] = 0x12
	i -= len(m.RunCommand)
	copy(dAtA[i:], m.RunCommand)
	i = encodeVarintGenerated(dAtA, i, uint64(len(m.RunCommand)))
	i--
	dAtA[i] = 0xa
	return len(dAtA) - i, nil
}

func (m *TaskStatus) Marshal() (dAtA []byte, err error) {
	size := m.Size()
	dAtA = make([]byte, size)
	n, err := m.MarshalToSizedBuffer(dAtA[:size])
	if err != nil {
		return nil, err
	}
	return dAtA[:n], nil
}

func (m *TaskStatus) MarshalTo(dAtA []byte) (int, error) {
	size := m.Size()
	return m.MarshalToSizedBuffer(dAtA[:size])
}

func (m *TaskStatus) MarshalToSizedBuffer(dAtA []byte) (int, error) {
	i := len(dAtA)
	_ = i
	var l int
	_ = l
	return len(dAtA) - i, nil
}

func encodeVarintGenerated(dAtA []byte, offset int, v uint64) int {
	offset -= sovGenerated(v)
	base := offset
	for v >= 1<<7 {
		dAtA[offset] = uint8(v&0x7f | 0x80)
		v >>= 7
		offset++
	}
	dAtA[offset] = uint8(v)
	return base
}
func (m *Task) Size() (n int) {
	if m == nil {
		return 0
	}
	var l int
	_ = l
	l = m.ObjectMeta.Size()
	n += 1 + l + sovGenerated(uint64(l))
	l = m.Spec.Size()
	n += 1 + l + sovGenerated(uint64(l))
	l = m.Status.Size()
	n += 1 + l + sovGenerated(uint64(l))
	return n
}

func (m *TaskEnvVar) Size() (n int) {
	if m == nil {
		return 0
	}
	var l int
	_ = l
	l = len(m.Name)
	n += 1 + l + sovGenerated(uint64(l))
	l = len(m.Value)
	n += 1 + l + sovGenerated(uint64(l))
	return n
}

func (m *TaskList) Size() (n int) {
	if m == nil {
		return 0
	}
	var l int
	_ = l
	l = m.ListMeta.Size()
	n += 1 + l + sovGenerated(uint64(l))
	if len(m.Items) > 0 {
		for _, e := range m.Items {
			l = e.Size()
			n += 1 + l + sovGenerated(uint64(l))
		}
	}
	return n
}

func (m *TaskSpec) Size() (n int) {
	if m == nil {
		return 0
	}
	var l int
	_ = l
	l = len(m.RunCommand)
	n += 1 + l + sovGenerated(uint64(l))
	l = len(m.ProjectId)
	n += 1 + l + sovGenerated(uint64(l))
	if len(m.Env) > 0 {
		for _, e := range m.Env {
			l = e.Size()
			n += 1 + l + sovGenerated(uint64(l))
		}
	}
	return n
}

func (m *TaskStatus) Size() (n int) {
	if m == nil {
		return 0
	}
	var l int
	_ = l
	return n
}

func sovGenerated(x uint64) (n int) {
	return (math_bits.Len64(x|1) + 6) / 7
}
func sozGenerated(x uint64) (n int) {
	return sovGenerated(uint64((x << 1) ^ uint64((int64(x) >> 63))))
}
func (this *Task) String() string {
	if this == nil {
		return "nil"
	}
	s := strings.Join([]string{`&Task{`,
		`ObjectMeta:` + strings.Replace(strings.Replace(fmt.Sprintf("%v", this.ObjectMeta), "ObjectMeta", "v1.ObjectMeta", 1), `&`, ``, 1) + `,`,
		`Spec:` + strings.Replace(strings.Replace(this.Spec.String(), "TaskSpec", "TaskSpec", 1), `&`, ``, 1) + `,`,
		`Status:` + strings.Replace(strings.Replace(this.Status.String(), "TaskStatus", "TaskStatus", 1), `&`, ``, 1) + `,`,
		`}`,
	}, "")
	return s
}
func (this *TaskEnvVar) String() string {
	if this == nil {
		return "nil"
	}
	s := strings.Join([]string{`&TaskEnvVar{`,
		`Name:` + fmt.Sprintf("%v", this.Name) + `,`,
		`Value:` + fmt.Sprintf("%v", this.Value) + `,`,
		`}`,
	}, "")
	return s
}
func (this *TaskList) String() string {
	if this == nil {
		return "nil"
	}
	repeatedStringForItems := "[]Task{"
	for _, f := range this.Items {
		repeatedStringForItems += strings.Replace(strings.Replace(f.String(), "Task", "Task", 1), `&`, ``, 1) + ","
	}
	repeatedStringForItems += "}"
	s := strings.Join([]string{`&TaskList{`,
		`ListMeta:` + strings.Replace(strings.Replace(fmt.Sprintf("%v", this.ListMeta), "ListMeta", "v1.ListMeta", 1), `&`, ``, 1) + `,`,
		`Items:` + repeatedStringForItems + `,`,
		`}`,
	}, "")
	return s
}
func (this *TaskSpec) String() string {
	if this == nil {
		return "nil"
	}
	repeatedStringForEnv := "[]TaskEnvVar{"
	for _, f := range this.Env {
		repeatedStringForEnv += strings.Replace(strings.Replace(f.String(), "TaskEnvVar", "TaskEnvVar", 1), `&`, ``, 1) + ","
	}
	repeatedStringForEnv += "}"
	s := strings.Join([]string{`&TaskSpec{`,
		`RunCommand:` + fmt.Sprintf("%v", this.RunCommand) + `,`,
		`ProjectId:` + fmt.Sprintf("%v", this.ProjectId) + `,`,
		`Env:` + repeatedStringForEnv + `,`,
		`}`,
	}, "")
	return s
}
func (this *TaskStatus) String() string {
	if this == nil {
		return "nil"
	}
	s := strings.Join([]string{`&TaskStatus{`,
		`}`,
	}, "")
	return s
}
func valueToStringGenerated(v interface{}) string {
	rv := reflect.ValueOf(v)
	if rv.IsNil() {
		return "nil"
	}
	pv := reflect.Indirect(rv).Interface()
	return fmt.Sprintf("*%v", pv)
}
func (m *Task) Unmarshal(dAtA []byte) error {
	l := len(dAtA)
	iNdEx := 0
	for iNdEx < l {
		preIndex := iNdEx
		var wire uint64
		for shift := uint(0); ; shift += 7 {
			if shift >= 64 {
				return ErrIntOverflowGenerated
			}
			if iNdEx >= l {
				return io.ErrUnexpectedEOF
			}
			b := dAtA[iNdEx]
			iNdEx++
			wire |= uint64(b&0x7F) << shift
			if b < 0x80 {
				break
			}
		}
		fieldNum := int32(wire >> 3)
		wireType := int(wire & 0x7)
		if wireType == 4 {
			return fmt.Errorf("proto: Task: wiretype end group for non-group")
		}
		if fieldNum <= 0 {
			return fmt.Errorf("proto: Task: illegal tag %d (wire type %d)", fieldNum, wire)
		}
		switch fieldNum {
		case 1:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field ObjectMeta", wireType)
			}
			var msglen int
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				msglen |= int(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			if msglen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + msglen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			if err := m.ObjectMeta.Unmarshal(dAtA[iNdEx:postIndex]); err != nil {
				return err
			}
			iNdEx = postIndex
		case 2:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field Spec", wireType)
			}
			var msglen int
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				msglen |= int(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			if msglen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + msglen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			if err := m.Spec.Unmarshal(dAtA[iNdEx:postIndex]); err != nil {
				return err
			}
			iNdEx = postIndex
		case 3:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field Status", wireType)
			}
			var msglen int
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				msglen |= int(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			if msglen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + msglen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			if err := m.Status.Unmarshal(dAtA[iNdEx:postIndex]); err != nil {
				return err
			}
			iNdEx = postIndex
		default:
			iNdEx = preIndex
			skippy, err := skipGenerated(dAtA[iNdEx:])
			if err != nil {
				return err
			}
			if (skippy < 0) || (iNdEx+skippy) < 0 {
				return ErrInvalidLengthGenerated
			}
			if (iNdEx + skippy) > l {
				return io.ErrUnexpectedEOF
			}
			iNdEx += skippy
		}
	}

	if iNdEx > l {
		return io.ErrUnexpectedEOF
	}
	return nil
}
func (m *TaskEnvVar) Unmarshal(dAtA []byte) error {
	l := len(dAtA)
	iNdEx := 0
	for iNdEx < l {
		preIndex := iNdEx
		var wire uint64
		for shift := uint(0); ; shift += 7 {
			if shift >= 64 {
				return ErrIntOverflowGenerated
			}
			if iNdEx >= l {
				return io.ErrUnexpectedEOF
			}
			b := dAtA[iNdEx]
			iNdEx++
			wire |= uint64(b&0x7F) << shift
			if b < 0x80 {
				break
			}
		}
		fieldNum := int32(wire >> 3)
		wireType := int(wire & 0x7)
		if wireType == 4 {
			return fmt.Errorf("proto: TaskEnvVar: wiretype end group for non-group")
		}
		if fieldNum <= 0 {
			return fmt.Errorf("proto: TaskEnvVar: illegal tag %d (wire type %d)", fieldNum, wire)
		}
		switch fieldNum {
		case 1:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field Name", wireType)
			}
			var stringLen uint64
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				stringLen |= uint64(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			intStringLen := int(stringLen)
			if intStringLen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + intStringLen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			m.Name = string(dAtA[iNdEx:postIndex])
			iNdEx = postIndex
		case 2:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field Value", wireType)
			}
			var stringLen uint64
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				stringLen |= uint64(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			intStringLen := int(stringLen)
			if intStringLen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + intStringLen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			m.Value = string(dAtA[iNdEx:postIndex])
			iNdEx = postIndex
		default:
			iNdEx = preIndex
			skippy, err := skipGenerated(dAtA[iNdEx:])
			if err != nil {
				return err
			}
			if (skippy < 0) || (iNdEx+skippy) < 0 {
				return ErrInvalidLengthGenerated
			}
			if (iNdEx + skippy) > l {
				return io.ErrUnexpectedEOF
			}
			iNdEx += skippy
		}
	}

	if iNdEx > l {
		return io.ErrUnexpectedEOF
	}
	return nil
}
func (m *TaskList) Unmarshal(dAtA []byte) error {
	l := len(dAtA)
	iNdEx := 0
	for iNdEx < l {
		preIndex := iNdEx
		var wire uint64
		for shift := uint(0); ; shift += 7 {
			if shift >= 64 {
				return ErrIntOverflowGenerated
			}
			if iNdEx >= l {
				return io.ErrUnexpectedEOF
			}
			b := dAtA[iNdEx]
			iNdEx++
			wire |= uint64(b&0x7F) << shift
			if b < 0x80 {
				break
			}
		}
		fieldNum := int32(wire >> 3)
		wireType := int(wire & 0x7)
		if wireType == 4 {
			return fmt.Errorf("proto: TaskList: wiretype end group for non-group")
		}
		if fieldNum <= 0 {
			return fmt.Errorf("proto: TaskList: illegal tag %d (wire type %d)", fieldNum, wire)
		}
		switch fieldNum {
		case 1:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field ListMeta", wireType)
			}
			var msglen int
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				msglen |= int(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			if msglen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + msglen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			if err := m.ListMeta.Unmarshal(dAtA[iNdEx:postIndex]); err != nil {
				return err
			}
			iNdEx = postIndex
		case 2:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field Items", wireType)
			}
			var msglen int
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				msglen |= int(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			if msglen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + msglen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			m.Items = append(m.Items, Task{})
			if err := m.Items[len(m.Items)-1].Unmarshal(dAtA[iNdEx:postIndex]); err != nil {
				return err
			}
			iNdEx = postIndex
		default:
			iNdEx = preIndex
			skippy, err := skipGenerated(dAtA[iNdEx:])
			if err != nil {
				return err
			}
			if (skippy < 0) || (iNdEx+skippy) < 0 {
				return ErrInvalidLengthGenerated
			}
			if (iNdEx + skippy) > l {
				return io.ErrUnexpectedEOF
			}
			iNdEx += skippy
		}
	}

	if iNdEx > l {
		return io.ErrUnexpectedEOF
	}
	return nil
}
func (m *TaskSpec) Unmarshal(dAtA []byte) error {
	l := len(dAtA)
	iNdEx := 0
	for iNdEx < l {
		preIndex := iNdEx
		var wire uint64
		for shift := uint(0); ; shift += 7 {
			if shift >= 64 {
				return ErrIntOverflowGenerated
			}
			if iNdEx >= l {
				return io.ErrUnexpectedEOF
			}
			b := dAtA[iNdEx]
			iNdEx++
			wire |= uint64(b&0x7F) << shift
			if b < 0x80 {
				break
			}
		}
		fieldNum := int32(wire >> 3)
		wireType := int(wire & 0x7)
		if wireType == 4 {
			return fmt.Errorf("proto: TaskSpec: wiretype end group for non-group")
		}
		if fieldNum <= 0 {
			return fmt.Errorf("proto: TaskSpec: illegal tag %d (wire type %d)", fieldNum, wire)
		}
		switch fieldNum {
		case 1:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field RunCommand", wireType)
			}
			var stringLen uint64
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				stringLen |= uint64(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			intStringLen := int(stringLen)
			if intStringLen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + intStringLen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			m.RunCommand = string(dAtA[iNdEx:postIndex])
			iNdEx = postIndex
		case 2:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field ProjectId", wireType)
			}
			var stringLen uint64
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				stringLen |= uint64(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			intStringLen := int(stringLen)
			if intStringLen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + intStringLen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			m.ProjectId = string(dAtA[iNdEx:postIndex])
			iNdEx = postIndex
		case 3:
			if wireType != 2 {
				return fmt.Errorf("proto: wrong wireType = %d for field Env", wireType)
			}
			var msglen int
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				msglen |= int(b&0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			if msglen < 0 {
				return ErrInvalidLengthGenerated
			}
			postIndex := iNdEx + msglen
			if postIndex < 0 {
				return ErrInvalidLengthGenerated
			}
			if postIndex > l {
				return io.ErrUnexpectedEOF
			}
			m.Env = append(m.Env, TaskEnvVar{})
			if err := m.Env[len(m.Env)-1].Unmarshal(dAtA[iNdEx:postIndex]); err != nil {
				return err
			}
			iNdEx = postIndex
		default:
			iNdEx = preIndex
			skippy, err := skipGenerated(dAtA[iNdEx:])
			if err != nil {
				return err
			}
			if (skippy < 0) || (iNdEx+skippy) < 0 {
				return ErrInvalidLengthGenerated
			}
			if (iNdEx + skippy) > l {
				return io.ErrUnexpectedEOF
			}
			iNdEx += skippy
		}
	}

	if iNdEx > l {
		return io.ErrUnexpectedEOF
	}
	return nil
}
func (m *TaskStatus) Unmarshal(dAtA []byte) error {
	l := len(dAtA)
	iNdEx := 0
	for iNdEx < l {
		preIndex := iNdEx
		var wire uint64
		for shift := uint(0); ; shift += 7 {
			if shift >= 64 {
				return ErrIntOverflowGenerated
			}
			if iNdEx >= l {
				return io.ErrUnexpectedEOF
			}
			b := dAtA[iNdEx]
			iNdEx++
			wire |= uint64(b&0x7F) << shift
			if b < 0x80 {
				break
			}
		}
		fieldNum := int32(wire >> 3)
		wireType := int(wire & 0x7)
		if wireType == 4 {
			return fmt.Errorf("proto: TaskStatus: wiretype end group for non-group")
		}
		if fieldNum <= 0 {
			return fmt.Errorf("proto: TaskStatus: illegal tag %d (wire type %d)", fieldNum, wire)
		}
		switch fieldNum {
		default:
			iNdEx = preIndex
			skippy, err := skipGenerated(dAtA[iNdEx:])
			if err != nil {
				return err
			}
			if (skippy < 0) || (iNdEx+skippy) < 0 {
				return ErrInvalidLengthGenerated
			}
			if (iNdEx + skippy) > l {
				return io.ErrUnexpectedEOF
			}
			iNdEx += skippy
		}
	}

	if iNdEx > l {
		return io.ErrUnexpectedEOF
	}
	return nil
}
func skipGenerated(dAtA []byte) (n int, err error) {
	l := len(dAtA)
	iNdEx := 0
	depth := 0
	for iNdEx < l {
		var wire uint64
		for shift := uint(0); ; shift += 7 {
			if shift >= 64 {
				return 0, ErrIntOverflowGenerated
			}
			if iNdEx >= l {
				return 0, io.ErrUnexpectedEOF
			}
			b := dAtA[iNdEx]
			iNdEx++
			wire |= (uint64(b) & 0x7F) << shift
			if b < 0x80 {
				break
			}
		}
		wireType := int(wire & 0x7)
		switch wireType {
		case 0:
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return 0, ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return 0, io.ErrUnexpectedEOF
				}
				iNdEx++
				if dAtA[iNdEx-1] < 0x80 {
					break
				}
			}
		case 1:
			iNdEx += 8
		case 2:
			var length int
			for shift := uint(0); ; shift += 7 {
				if shift >= 64 {
					return 0, ErrIntOverflowGenerated
				}
				if iNdEx >= l {
					return 0, io.ErrUnexpectedEOF
				}
				b := dAtA[iNdEx]
				iNdEx++
				length |= (int(b) & 0x7F) << shift
				if b < 0x80 {
					break
				}
			}
			if length < 0 {
				return 0, ErrInvalidLengthGenerated
			}
			iNdEx += length
		case 3:
			depth++
		case 4:
			if depth == 0 {
				return 0, ErrUnexpectedEndOfGroupGenerated
			}
			depth--
		case 5:
			iNdEx += 4
		default:
			return 0, fmt.Errorf("proto: illegal wireType %d", wireType)
		}
		if iNdEx < 0 {
			return 0, ErrInvalidLengthGenerated
		}
		if depth == 0 {
			return iNdEx, nil
		}
	}
	return 0, io.ErrUnexpectedEOF
}

var (
	ErrInvalidLengthGenerated        = fmt.Errorf("proto: negative length found during unmarshaling")
	ErrIntOverflowGenerated          = fmt.Errorf("proto: integer overflow")
	ErrUnexpectedEndOfGroupGenerated = fmt.Errorf("proto: unexpected end of group")
)
