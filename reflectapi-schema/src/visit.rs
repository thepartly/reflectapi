use std::ops::ControlFlow;

use crate::{
    Enum, Field, Function, Primitive, Schema, Struct, Type, TypeParameter, TypeReference,
    Typespace, Variant,
};

pub trait Zero {
    const ZERO: Self;
}

impl Zero for usize {
    const ZERO: Self = 0;
}

impl Combine for usize {
    fn combine(self, other: Self) -> Self {
        self + other
    }
}

impl Zero for () {
    const ZERO: Self = ();
}

impl Combine for () {
    fn combine(self, _other: Self) -> Self {
        ()
    }
}

/// Associative operation for combining two values.
pub trait Combine: Zero {
    fn combine(self, other: Self) -> Self;
}

/// A trait for traversing a `Schema` and its children.
pub trait Visitor: Sized {
    // Would be nice to just use `std::ops::Add` but not implemented on `()`
    type Output: Combine;

    fn visit_schema_inputs(&mut self, s: &mut Schema) -> ControlFlow<Self::Output, Self::Output> {
        ControlFlow::Continue(
            s.input_types.visit_mut(self)?.combine(
                s.functions
                    .iter_mut()
                    .try_fold(Self::Output::ZERO, |acc, f| {
                        ControlFlow::Continue(acc.combine(self.visit_function_inputs(f)?))
                    })?,
            ),
        )
    }

    fn visit_schema_outputs(&mut self, s: &mut Schema) -> ControlFlow<Self::Output, Self::Output> {
        ControlFlow::Continue(
            s.output_types.visit_mut(self)?.combine(
                s.functions
                    .iter_mut()
                    .try_fold(Self::Output::ZERO, |acc, f| {
                        ControlFlow::Continue(acc.combine(self.visit_function_outputs(f)?))
                    })?,
            ),
        )
    }

    fn visit_function_inputs(
        &mut self,
        f: &mut Function,
    ) -> ControlFlow<Self::Output, Self::Output> {
        let mut acc = Self::Output::ZERO;
        if let Some(input_type) = &mut f.input_type {
            acc = acc.combine(self.visit_type_ref(input_type)?);
        }

        if let Some(input_headers) = &mut f.input_headers {
            acc = acc.combine(self.visit_type_ref(input_headers)?);
        }

        ControlFlow::Continue(acc)
    }

    fn visit_function_outputs(
        &mut self,
        f: &mut Function,
    ) -> ControlFlow<Self::Output, Self::Output> {
        let mut acc = Self::Output::ZERO;
        if let Some(output_type) = &mut f.output_type {
            acc = acc.combine(self.visit_type_ref(output_type)?);
        }

        if let Some(output_headers) = &mut f.error_type {
            acc = acc.combine(self.visit_type_ref(output_headers)?);
        }

        ControlFlow::Continue(acc)
    }

    fn visit_type(&mut self, t: &mut Type) -> ControlFlow<Self::Output, Self::Output> {
        t.visit_mut(self)
    }

    fn visit_enum(&mut self, e: &mut Enum) -> ControlFlow<Self::Output, Self::Output> {
        e.visit_mut(self)
    }

    fn visit_variant(&mut self, v: &mut Variant) -> ControlFlow<Self::Output, Self::Output> {
        v.visit_mut(self)
    }

    fn visit_struct(&mut self, s: &mut Struct) -> ControlFlow<Self::Output, Self::Output> {
        s.visit_mut(self)
    }

    fn visit_primitive(&mut self, p: &mut Primitive) -> ControlFlow<Self::Output, Self::Output> {
        p.visit_mut(self)
    }

    fn visit_type_parameter(
        &mut self,
        _p: &mut TypeParameter,
    ) -> ControlFlow<Self::Output, Self::Output> {
        ControlFlow::Continue(Self::Output::ZERO)
    }

    fn visit_field(&mut self, f: &mut Field) -> ControlFlow<Self::Output, Self::Output> {
        f.visit_mut(self)
    }

    fn visit_type_ref(
        &mut self,
        type_ref: &mut TypeReference,
    ) -> ControlFlow<Self::Output, Self::Output> {
        type_ref.visit_mut(self)
    }

    // Only called for `Struct`, `Enum`, `Primitive`, and `TypeReference` names
    fn visit_top_level_name(
        &mut self,
        _name: &mut String,
    ) -> ControlFlow<Self::Output, Self::Output> {
        ControlFlow::Continue(Self::Output::ZERO)
    }
}

pub trait VisitMut {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output>;
}

impl VisitMut for Typespace {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        self.invalidate_types_map();
        self.types.iter_mut().try_fold(V::Output::ZERO, |acc, t| {
            ControlFlow::Continue(acc.combine(visitor.visit_type(t)?))
        })
    }
}

impl VisitMut for Type {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        match self {
            Type::Struct(s) => visitor.visit_struct(s),
            Type::Enum(e) => visitor.visit_enum(e),
            Type::Primitive(p) => visitor.visit_primitive(p),
        }
    }
}

impl VisitMut for Struct {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        let ps = self
            .parameters
            .iter_mut()
            .try_fold(V::Output::ZERO, |acc, p| {
                ControlFlow::Continue(acc.combine(visitor.visit_type_parameter(p)?))
            })?;

        let fs = self.fields.iter_mut().try_fold(V::Output::ZERO, |acc, f| {
            ControlFlow::Continue(acc.combine(visitor.visit_field(f)?))
        })?;

        ControlFlow::Continue(
            ps.combine(fs)
                .combine(visitor.visit_top_level_name(&mut self.name)?),
        )
    }
}

impl VisitMut for Enum {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        let ps = self
            .parameters
            .iter_mut()
            .try_fold(V::Output::ZERO, |acc, p| {
                ControlFlow::Continue(acc.combine(visitor.visit_type_parameter(p)?))
            })?;

        let vs = self
            .variants
            .iter_mut()
            .try_fold(V::Output::ZERO, |acc, v| {
                ControlFlow::Continue(acc.combine(visitor.visit_variant(v)?))
            })?;

        ControlFlow::Continue(
            ps.combine(vs)
                .combine(visitor.visit_top_level_name(&mut self.name)?),
        )
    }
}

impl VisitMut for Variant {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        self.fields.iter_mut().try_fold(V::Output::ZERO, |acc, f| {
            ControlFlow::Continue(acc.combine(visitor.visit_field(f)?))
        })
    }
}

impl VisitMut for Primitive {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        ControlFlow::Continue(
            self.parameters
                .iter_mut()
                .try_fold(V::Output::ZERO, |acc, p| {
                    ControlFlow::Continue(acc.combine(visitor.visit_type_parameter(p)?))
                })?
                .combine(visitor.visit_top_level_name(&mut self.name)?),
        )
    }
}

impl VisitMut for Field {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        visitor.visit_type_ref(&mut self.type_ref)
    }
}

impl VisitMut for TypeReference {
    fn visit_mut<V: Visitor>(&mut self, visitor: &mut V) -> ControlFlow<V::Output, V::Output> {
        ControlFlow::Continue(
            self.arguments
                .iter_mut()
                .try_fold(V::Output::ZERO, |acc, a| {
                    ControlFlow::Continue(acc.combine(visitor.visit_type_ref(a)?))
                })?
                .combine(visitor.visit_top_level_name(&mut self.name)?),
        )
    }
}
