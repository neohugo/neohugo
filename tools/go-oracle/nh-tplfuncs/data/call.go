package main

import (
	"context"
	"errors"
	"fmt"
	"reflect"
)

var contextType = reflect.TypeFor[context.Context]()

// canBeNil reports whether an untyped nil can be assigned to the type (Go
// text/template exec.go).
func canBeNil(typ reflect.Type) bool {
	switch typ.Kind() {
	case reflect.Chan, reflect.Func, reflect.Interface, reflect.Map, reflect.Ptr, reflect.Slice:
		return true
	}
	return false
}

// validateType is text/template's check of an argument against a typed
// parameter (the only typed parameters of these namespaces are strings).
func validateType(value any, typ reflect.Type) (reflect.Value, error) {
	v := reflect.ValueOf(value)
	if !v.IsValid() {
		if canBeNil(typ) {
			return reflect.Zero(typ), nil
		}
		return reflect.Value{}, fmt.Errorf("invalid value; expected %s", typ)
	}
	if !v.Type().AssignableTo(typ) {
		return reflect.Value{}, fmt.Errorf("wrong type for value; expected %s; got %s", typ, v.Type())
	}
	return v, nil
}

// call calls the method name of recv the way text/template calls a
// function: the context is injected when the first parameter is a
// context.Context, the argument count and typed parameters are checked with
// Go's messages, a panic becomes the error (safeCall) and a nil interface
// result is nil.
func call(ctx context.Context, recv any, name string, args []any) (res any, err error) {
	defer func() {
		if r := recover(); r != nil {
			if e, ok := r.(error); ok {
				err = e
			} else {
				err = fmt.Errorf("%v", r)
			}
		}
	}()
	m := reflect.ValueOf(recv).MethodByName(name)
	if !m.IsValid() {
		return nil, fmt.Errorf("no method %s", name)
	}
	typ := m.Type()
	var in []reflect.Value
	start := 0
	if typ.NumIn() > 0 && typ.In(0) == contextType {
		in = append(in, reflect.ValueOf(ctx))
		start = 1
	}
	numFixed := typ.NumIn() - start
	if typ.IsVariadic() {
		numFixed--
		if len(args) < numFixed {
			return nil, fmt.Errorf("wrong number of args for %s: want at least %d got %d", name, numFixed, len(args))
		}
	} else if len(args) != numFixed {
		return nil, fmt.Errorf("wrong number of args for %s: want %d got %d", name, numFixed, len(args))
	}
	for i, a := range args {
		var t reflect.Type
		if i < numFixed {
			t = typ.In(start + i)
		} else {
			t = typ.In(typ.NumIn() - 1).Elem()
		}
		v, err := validateType(a, t)
		if err != nil {
			return nil, err
		}
		in = append(in, v)
	}
	outs := m.Call(in)
	switch len(outs) {
	case 0:
		return nil, errors.New("no result")
	case 2:
		if !outs[1].IsNil() {
			return nil, outs[1].Interface().(error)
		}
	}
	return outs[0].Interface(), nil
}
