use crate::code_writer::CodeWriter;
use crate::field::FieldElem;
use crate::field::FieldGen;
use crate::field::FieldKind;
use crate::field::MapField;
use crate::field::OptionKind;
use crate::field::RepeatedField;
use crate::field::RepeatedFieldKind;
use crate::field::SingularField;
use crate::field::SingularFieldFlag;
use crate::inside::protobuf_crate_path;
use crate::oneof::OneofField;
use crate::rust_types_values::ProtobufTypeGen;
use crate::rust_types_values::RustType;
use protobuf::descriptorx::WithScope;

struct AccessorFn {
    name: String,
    // function type params after first underscore
    type_params: Vec<String>,
    callback_params: Vec<String>,
}

impl AccessorFn {
    fn sig(&self) -> String {
        let mut s = self.name.clone();
        s.push_str("::<_");
        for p in &self.type_params {
            s.push_str(", ");
            s.push_str(&p);
        }
        s.push_str(">");
        s
    }
}

impl FieldGen<'_> {
    fn make_accessor_fns_lambda(&self) -> Vec<String> {
        let message = self.proto_field.message.rust_name();
        vec![
            format!("|m: &{}| {{ &m.{} }}", message, self.rust_name),
            format!("|m: &mut {}| {{ &mut m.{} }}", message, self.rust_name),
        ]
    }

    fn make_accessor_fns_has_get(&self) -> Vec<String> {
        let message = self.proto_field.message.rust_name();
        vec![
            format!("{}::has_{}", message, self.rust_name),
            format!("{}::get_{}", message, self.rust_name),
        ]
    }

    fn make_accessor_fns_has_get_set(&self) -> Vec<String> {
        let message = self.proto_field.message.rust_name();
        vec![
            format!("{}::has_{}", message, self.rust_name),
            format!("{}::get_{}", message, self.rust_name),
            format!("{}::set_{}", message, self.rust_name),
        ]
    }

    fn make_accessor_fns_has_get_mut_set(&self) -> Vec<String> {
        let message = self.proto_field.message.rust_name();
        vec![
            format!("{}::has_{}", message, self.rust_name),
            format!("{}::get_{}", message, self.rust_name),
            format!("{}::mut_{}", message, self.rust_name),
            format!("{}::set_{}", message, self.rust_name),
        ]
    }

    fn accessor_fn_map(&self, map_field: &MapField) -> AccessorFn {
        let MapField {
            ref key, ref value, ..
        } = map_field;
        AccessorFn {
            name: "make_map_accessor".to_owned(),
            type_params: vec![
                key.lib_protobuf_type(&self.customize),
                value.lib_protobuf_type(&self.customize),
            ],
            callback_params: self.make_accessor_fns_lambda(),
        }
    }

    fn accessor_fn_repeated(&self, repeated_field: &RepeatedField) -> AccessorFn {
        let RepeatedField { ref elem, .. } = repeated_field;
        let name = match repeated_field.kind() {
            RepeatedFieldKind::Vec => "make_vec_accessor",
            RepeatedFieldKind::RepeatedField => "make_repeated_field_accessor",
        };
        AccessorFn {
            name: name.to_owned(),
            type_params: vec![elem.lib_protobuf_type(&self.customize)],
            callback_params: self.make_accessor_fns_lambda(),
        }
    }

    fn accessor_fn_singular_without_flag(&self, elem: &FieldElem) -> AccessorFn {
        if let &FieldElem::Message(ref name, ..) = elem {
            // TODO: old style, needed because of default instance

            AccessorFn {
                name: "make_singular_message_accessor".to_owned(),
                type_params: vec![name.0.clone()],
                callback_params: self.make_accessor_fns_has_get(),
            }
        } else {
            AccessorFn {
                name: "make_simple_field_accessor".to_owned(),
                type_params: vec![elem.lib_protobuf_type(&self.customize)],
                callback_params: self.make_accessor_fns_lambda(),
            }
        }
    }

    fn accessor_fn_singular_with_flag(
        &self,
        elem: &FieldElem,
        option_kind: OptionKind,
    ) -> AccessorFn {
        let name = match option_kind {
            OptionKind::Option => "make_option_accessor",
            OptionKind::SingularField => "make_singular_field_accessor",
            OptionKind::SingularPtrField => "make_singular_ptr_field_accessor",
            OptionKind::_OptionBox => unreachable!(),
        };
        AccessorFn {
            name: name.to_owned(),
            type_params: vec![elem.lib_protobuf_type(&self.customize)],
            callback_params: self.make_accessor_fns_lambda(),
        }
    }

    fn accessor_fn_oneof(&self, oneof: &OneofField) -> AccessorFn {
        let OneofField { ref elem, .. } = oneof;
        // TODO: uses old style

        if let FieldElem::Enum(ref name, ..) = &oneof.elem {
            return AccessorFn {
                name: "make_oneof_copy_has_get_set_accessors".to_owned(),
                type_params: vec![ProtobufTypeGen::Enum(name.clone()).rust_type(&self.customize)],
                callback_params: self.make_accessor_fns_has_get_set(),
            };
        }

        if elem.is_copy() {
            return AccessorFn {
                name: "make_oneof_copy_has_get_set_accessors".to_owned(),
                type_params: vec![elem.protobuf_type_gen().rust_type(&self.customize)],
                callback_params: self.make_accessor_fns_has_get_set(),
            };
        }

        if let FieldElem::Message(ref name, ..) = elem {
            return AccessorFn {
                name: "make_oneof_message_has_get_mut_set_accessor".to_owned(),
                type_params: vec![format!("{}", name)],
                callback_params: self.make_accessor_fns_has_get_mut_set(),
            };
        }

        // string or bytes
        AccessorFn {
            name: "make_oneof_deref_has_get_set_accessor".to_owned(),
            type_params: vec![elem.protobuf_type_gen().rust_type(&self.customize)],
            callback_params: self.make_accessor_fns_has_get_set(),
        }
    }

    fn accessor_fn(&self) -> AccessorFn {
        match self.kind {
            FieldKind::Repeated(ref repeated_field) => self.accessor_fn_repeated(repeated_field),
            FieldKind::Map(ref map_field) => self.accessor_fn_map(map_field),
            FieldKind::Singular(SingularField {
                ref elem,
                flag: SingularFieldFlag::WithoutFlag,
            }) => self.accessor_fn_singular_without_flag(elem),
            FieldKind::Singular(SingularField {
                ref elem,
                flag: SingularFieldFlag::WithFlag { option_kind, .. },
            }) => self.accessor_fn_singular_with_flag(elem, option_kind),
            FieldKind::Oneof(ref oneof) => self.accessor_fn_oneof(oneof),
        }
    }

    pub fn write_descriptor_field(&self, fields_var: &str, w: &mut CodeWriter) {
        let accessor_fn = self.accessor_fn();
        w.write_line(&format!(
            "{}.push({}::reflect::rt::{}(",
            fields_var,
            protobuf_crate_path(&self.customize),
            accessor_fn.sig()
        ));
        w.indented(|w| {
            w.write_line(&format!("\"{}\",", self.proto_field.name()));
            for callback in &accessor_fn.callback_params {
                w.write_line(&format!("{},", callback));
            }
        });
        w.write_line("));");
    }
}