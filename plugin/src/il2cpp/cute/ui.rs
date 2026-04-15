use crate::il2cpp::{
    helper::{get_class_from_namespace, get_pointer_field},
    types::*,
};

pub struct AtlasReference {
    instance: *mut Il2CppObject,
}

impl AtlasReference {
    pub fn new(atlas_reference: *mut Il2CppObject) -> Self {
        return AtlasReference {
            instance: atlas_reference,
        };
    }

    pub fn get_sprites(&self) -> *mut Il2CppArray {
        let class = get_class_from_namespace("Cute.UI.Assembly.dll", "Cute.UI", "AtlasReference");
        return get_pointer_field(class, "sprites", self.instance) as *mut Il2CppArray;
    }
}
