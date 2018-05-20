use std::collections::HashMap;
use std::marker;

use Message;

use descriptor::FieldDescriptorProto_Label;
use descriptor::FieldDescriptorProto_Type;
use descriptor::FieldDescriptorProto;
use descriptor::DescriptorProto;
use descriptor::FileDescriptorProto;

use descriptorx::find_message_by_rust_name;

use reflect::EnumValueDescriptor;
use reflect::ReflectValueBox;
use reflect::ReflectValueRef;
use reflect::EnumDescriptor;
use reflect::repeated::ReflectRepeatedRef;
use reflect::map::ReflectMapRef;
use reflect::accessor::FieldAccessor;
use reflect::accessor::AccessorKind;
use reflect::accessor::singular::SingularFieldAccessor;
use reflect::accessor::repeated::RepeatedFieldAccessor;
use reflect::repeated::ReflectRepeatedMut;


pub struct FieldDescriptor {
    proto: &'static FieldDescriptorProto,
    accessor: FieldAccessor,
}

impl FieldDescriptor {
    fn new(
        accessor: FieldAccessor,
        proto: &'static FieldDescriptorProto,
    ) -> FieldDescriptor {
        assert_eq!(proto.get_name(), accessor.name);
        FieldDescriptor {
            proto,
            accessor,
        }
    }

    pub fn proto(&self) -> &'static FieldDescriptorProto {
        self.proto
    }

    pub fn name(&self) -> &'static str {
        self.accessor.name
    }

    pub fn is_repeated(&self) -> bool {
        self.proto.get_label() == FieldDescriptorProto_Label::LABEL_REPEATED
    }

    /// Return enum descriptor for enum field, panics if field type is not enum.
    pub fn enum_descriptor(&self) -> &'static EnumDescriptor {
        match self.accessor.accessor {
            AccessorKind::Singular(ref a) => a.enum_descriptor(),
            AccessorKind::Repeated(ref a) => a.enum_descriptor(),
            _ => panic!("not a singular or repeated field"),
        }
    }

    /// Return enum descriptor for message field, panics if field type is not message.
    pub fn message_descriptor(&self) -> &'static MessageDescriptor {
        match self.accessor.accessor {
            AccessorKind::Singular(ref a) => a.message_descriptor(),
            AccessorKind::Repeated(ref a) => a.message_descriptor(),
            _ => panic!("not a singular or repeated field"),
        }
    }

    pub fn has_field(&self, m: &Message) -> bool {
        match self.accessor.accessor {
            AccessorKind::Singular(ref a) => a.has_field_generic(m),
            AccessorKind::Repeated(ref a) => a.len_field_generic(m) != 0,
            AccessorKind::Map(ref a) => a.len_field_generic(m) != 0,
        }
    }

    pub fn len_field(&self, m: &Message) -> usize {
        match self.accessor.accessor {
            AccessorKind::Singular(ref a) => if a.has_field_generic(m) { 1 } else { 0 },
            AccessorKind::Repeated(ref a) => a.len_field_generic(m),
            AccessorKind::Map(ref a) => a.len_field_generic(m),
        }
    }

    fn singular(&self) -> &SingularFieldAccessor {
        match self.accessor.accessor {
            AccessorKind::Singular(ref a) => &**a,
            _ => panic!("not a singular field"),
        }
    }

    fn repeated(&self) -> &RepeatedFieldAccessor {
        match self.accessor.accessor {
            AccessorKind::Repeated(ref a) => &**a,
            _ => panic!("not a repeated field"),
        }
    }

    /// Get message field or default instance if field is unset.
    /// Panic if field type is not message.
    pub fn get_message<'a>(&self, m: &'a Message) -> &'a Message {
        match self.singular().get_message_generic(m) {
            Some(m) => m,
            None => self.message_descriptor().default_instance(),
        }
    }

    pub fn mut_message<'a>(&self, m: &'a mut Message) -> &'a mut Message {
        self.singular().mut_message_generic(m)
    }

    pub fn get_enum(&self, m: &Message) -> &'static EnumValueDescriptor {
        assert_eq!(FieldDescriptorProto_Type::TYPE_ENUM, self.proto.get_field_type());
        self.singular().get_enum_generic(m)
    }

    pub fn get_str<'a>(&self, m: &'a Message) -> &'a str {
        assert_eq!(FieldDescriptorProto_Type::TYPE_STRING, self.proto.get_field_type());
        self.singular().get_str_generic(m)
    }

    pub fn get_bytes<'a>(&self, m: &'a Message) -> &'a [u8] {
        assert_eq!(FieldDescriptorProto_Type::TYPE_BYTES, self.proto.get_field_type());
        self.singular().get_bytes_generic(m)
    }

    pub fn get_u32(&self, m: &Message) -> u32 {
        self.singular().get_u32_generic(m)
    }

    pub fn get_u64(&self, m: &Message) -> u64 {
        self.singular().get_u64_generic(m)
    }

    pub fn get_i32(&self, m: &Message) -> i32 {
        self.singular().get_i32_generic(m)
    }

    pub fn get_i64(&self, m: &Message) -> i64 {
        self.singular().get_i64_generic(m)
    }

    pub fn get_bool(&self, m: &Message) -> bool {
        assert_eq!(FieldDescriptorProto_Type::TYPE_BOOL, self.proto.get_field_type());
        self.singular().get_bool_generic(m)
    }

    pub fn get_f32(&self, m: &Message) -> f32 {
        self.singular().get_f32_generic(m)
    }

    pub fn get_f64(&self, m: &Message) -> f64 {
        self.singular().get_f64_generic(m)
    }

    pub fn get_singular_field_or_default<'a>(&self, m: &'a Message) -> ReflectValueRef<'a> {
        self.singular().get_singular_field_or_default(m)
    }

    pub fn get_reflect<'a>(&self, m: &'a Message) -> ReflectFieldRef<'a> {
        match self.accessor.accessor {
            AccessorKind::Singular(ref a) => ReflectFieldRef::Optional(a.get_reflect(m)),
            AccessorKind::Repeated(ref a) => ReflectFieldRef::Repeated(a.get_reflect(m)),
            AccessorKind::Map(ref a) => ReflectFieldRef::Map(a.get_reflect(m)),
        }
    }

    pub fn get_repeated<'a>(&self, m: &'a Message) -> ReflectRepeatedRef<'a> {
        self.repeated().get_reflect(m)
    }

    pub fn mut_repeated<'a>(&mut self, m: &'a mut Message) -> ReflectRepeatedMut<'a> {
        self.repeated().mut_reflect(m)
    }

    pub fn set_singular_field(&self, m: &mut Message, value: ReflectValueBox) {
        self.singular().set_singular_field(m, value)
    }
}


trait MessageFactory {
    fn new_instance(&self) -> Box<Message>;
    fn default_instance(&self) -> &Message;
    fn clone(&self, message: &Message) -> Box<Message>;
}

struct MessageFactoryImpl<M>(marker::PhantomData<M>);

impl<M> MessageFactory for MessageFactoryImpl<M>
    where M : 'static + Message + Default + Clone
{
    fn new_instance(&self) -> Box<Message> {
        let m: M = Default::default();
        Box::new(m)
    }

    fn default_instance(&self) -> &Message {
        M::default_instance() as &Message
    }

    fn clone(&self, message: &Message) -> Box<Message> {
        let m: &M = message.as_any().downcast_ref().expect("wrong message type");
        Box::new(m.clone())
    }
}

pub struct MessageDescriptor {
    full_name: String,
    proto: &'static DescriptorProto,
    factory: &'static MessageFactory,
    fields: Vec<FieldDescriptor>,

    index_by_name: HashMap<String, usize>,
    index_by_number: HashMap<u32, usize>,
}

impl MessageDescriptor {
    pub fn for_type<M : Message>() -> &'static MessageDescriptor {
        M::descriptor_static()
    }

    pub fn new<M : 'static + Message + Default + Clone>(
        rust_name: &'static str,
        fields: Vec<FieldAccessor>,
        file: &'static FileDescriptorProto,
    ) -> MessageDescriptor {
        let proto = find_message_by_rust_name(file, rust_name);

        let mut field_proto_by_name = HashMap::new();
        for field_proto in proto.message.get_field() {
            field_proto_by_name.insert(field_proto.get_name(), field_proto);
        }

        let mut index_by_name = HashMap::new();
        let mut index_by_number = HashMap::new();
        for (i, f) in proto.message.get_field().iter().enumerate() {
            index_by_number.insert(f.get_number() as u32, i);
            index_by_name.insert(f.get_name().to_string(), i);
        }

        let mut full_name = file.get_package().to_string();
        if full_name.len() > 0 {
            full_name.push('.');
        }
        full_name.push_str(proto.message.get_name());

        MessageDescriptor {
            full_name: full_name,
            proto: proto.message,
            factory: &MessageFactoryImpl(marker::PhantomData::<M>),
            fields: fields
                .into_iter()
                .map(|f| {
                    let proto = *field_proto_by_name.get(f.name).unwrap();
                    FieldDescriptor::new(f, proto)
                })
                .collect(),
            index_by_name: index_by_name,
            index_by_number: index_by_number,
        }
    }

    /// New empty message
    pub fn new_instance(&self) -> Box<Message> {
        self.factory.new_instance()
    }

    /// Shared immutable empty message
    pub fn default_instance(&self) -> &Message {
        self.factory.default_instance()
    }

    /// Clone a message
    pub fn clone(&self, message: &Message) -> Box<Message> {
        self.factory.clone(message)
    }

    pub fn name(&self) -> &'static str {
        self.proto.get_name()
    }

    pub fn full_name(&self) -> &str {
        &self.full_name[..]
    }

    pub fn fields<'a>(&'a self) -> &'a [FieldDescriptor] {
        &self.fields
    }

    pub fn field_by_name<'a>(&'a self, name: &str) -> &'a FieldDescriptor {
        let &index = self.index_by_name.get(name).unwrap();
        &self.fields[index]
    }

    pub fn field_by_number<'a>(&'a self, number: u32) -> &'a FieldDescriptor {
        let &index = self.index_by_number.get(&number).unwrap();
        &self.fields[index]
    }

    pub fn cast<M : 'static>(&self, message: Box<Message>) -> Result<M, Box<Message>> {
        message.downcast_box::<M>().map(|m| *m)
    }
}

/// Reference to a value stored in a field, optional, repeated or map.
pub enum ReflectFieldRef<'a> {
    /// Singular field, optional or required in proto3 and just plain field in proto3
    Optional(Option<ReflectValueRef<'a>>),
    /// Repeated field
    Repeated(ReflectRepeatedRef<'a>),
    /// Map field
    Map(ReflectMapRef<'a>),
}

fn _assert_sync<'a>() {
    fn _assert_send_sync<T : Sync>() {}
    _assert_send_sync::<ReflectFieldRef<'a>>();
}
