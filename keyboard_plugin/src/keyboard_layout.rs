use dbus::arg::{Append, Arg, ArgType, Get};
use dbus::{arg, Signature};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct KeyboardLayout {
    pub description: String,
    pub name: String,
    pub variant: Option<String>,
}

impl Append for KeyboardLayout {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        let variant = if let Some(variant) = self.variant.clone() {
            variant
        } else {
            String::from("None")
        };

        iter.append_struct(|i| {
            i.append(self.description.clone());
            i.append(self.name.clone());
            i.append(variant);
        });
    }
}

impl<'a> Get<'a> for KeyboardLayout {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (description, name, variant) = <(String, String, String)>::get(i)?;
        Some(Self {
            description,
            name,
            variant: if variant == "None" {
                None
            } else {
                Some(variant)
            },
        })
    }
}

impl Arg for KeyboardLayout {
    const ARG_TYPE: ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(sss)\0") }
    }
}
