use std::{
    ffi::{c_float, c_void},
    ptr::null,
};

use serde::Serialize;

use crate::{
    array_get_byte,
    il2cpp::{helper::*, types::*},
};

#[derive(Serialize)]
pub struct Rect {
    pub x: c_float,
    pub y: c_float,
    pub width: c_float,
    pub height: c_float,
}

pub fn texture_to_texture2d(texture: *mut Il2CppObject) -> *mut Il2CppObject {
    unsafe {
        let texture2d_class =
            get_class_from_image("UnityEngine.CoreModule.dll", "UnityEngine.Texture2D");
        let rendertexture_class =
            get_class_from_image("UnityEngine.CoreModule.dll", "UnityEngine.RenderTexture");
        let texture_save_loader_util_class = get_gallop_class("TextureSaveLoaderUtil");

        let render_texture_get_temporary: unsafe extern "C" fn(i32, i32) -> *mut Il2CppObject =
            std::mem::transmute(get_method(rendertexture_class, "GetTemporary", 2));
        let render_texture_release: unsafe extern "C" fn(*mut Il2CppObject) =
            std::mem::transmute(get_method(rendertexture_class, "Release", 0));
        let blit2: unsafe extern "C" fn(*mut Il2CppObject, *mut Il2CppObject) =
            std::mem::transmute(resolve_icall(
                "UnityEngine.Graphics::Blit2(UnityEngine.Texture,UnityEngine.RenderTexture)",
            ));
        let convert_render_tex_to_texture2d: unsafe extern "C" fn(
            *mut Il2CppObject,
            i32,
        ) -> *mut Il2CppObject = std::mem::transmute(get_method(
            texture_save_loader_util_class,
            "ConvertRenderTexToTexture2D",
            2,
        ));

        let width = get_i32(texture2d_class, "width", texture);
        let height = get_i32(texture2d_class, "height", texture);
        let render_texture = render_texture_get_temporary(width, height);

        blit2(texture, render_texture);
        let out = convert_render_tex_to_texture2d(render_texture, 4);
        render_texture_release(render_texture);
        return out;
    }
}

pub fn texture2d_to_png(texture2d: *mut Il2CppObject) -> Vec<u8> {
    unsafe {
        let image_conversion_class = get_class_from_image(
            "UnityEngine.ImageConversionModule.dll",
            "UnityEngine.ImageConversion",
        );

        let encode_to_png: unsafe extern "C" fn(
            *mut Il2CppObject,
            *const MethodInfo,
        ) -> *mut Il2CppArray =
            std::mem::transmute(get_method(image_conversion_class, "EncodeToPNG", 1));

        let png_byte_array = encode_to_png(texture2d, null());
        let mut png_byte_vec = Vec::new();
        for i in 0..(*png_byte_array).max_length {
            png_byte_vec.push(array_get_byte(png_byte_array.as_ref_unchecked(), i));
        }
        return png_byte_vec;
    }
}

pub fn get_total_rank(total: i32) -> i32 {
    unsafe {
        let class = get_gallop_class("SingleModeDefine");
        type GetTotalRank = unsafe extern "C" fn(i32, *const c_void) -> i32;
        let func: GetTotalRank = std::mem::transmute(get_method(class, "GetTotalRank", 1));
        return func(total, null());
    }
}

pub fn get_final_training_rank_sprite(trained_rank: i32) -> *mut Il2CppObject {
    unsafe {
        let gallop_util_class = get_gallop_class("GallopUtil");

        let get_sprite: unsafe extern "C" fn(i32, *const c_void) -> *mut Il2CppObject =
            std::mem::transmute(get_method(
                gallop_util_class,
                "GetFinalTrainingRankSprite",
                1,
            ));

        return get_sprite(trained_rank, null());
    }
}

pub fn rank_score_to_rect(rank_score: i32) -> Rect {
    let rank = get_total_rank(rank_score);
    let sprite = get_final_training_rank_sprite(rank);
    return rect_from_sprite(sprite);
}

pub fn get_sprite_texture2d(sprite: *mut Il2CppObject) -> *mut Il2CppObject {
    unsafe {
        let sprite_class = get_class_from_image("UnityEngine.CoreModule.dll", "UnityEngine.Sprite");

        let sprite_get_texture: unsafe extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
            std::mem::transmute(get_method(sprite_class, "get_texture", 0));

        return sprite_get_texture(sprite);
    }
}

pub fn rect_from_sprite(sprite: *mut Il2CppObject) -> Rect {
    unsafe {
        let sprite_class = get_class_from_image("UnityEngine.CoreModule.dll", "UnityEngine.Sprite");
        let rect_class = get_class_from_image("UnityEngine.CoreModule.dll", "UnityEngine.Rect");

        let sprite_get_texturerect: unsafe extern "C" fn(
            *mut Il2CppObject,
            *mut Il2CppObject,
        ) -> *mut Il2CppObject =
            std::mem::transmute(get_method(sprite_class, "get_textureRect", 0));

        let rect = object_new(rect_class);
        let texturerect = sprite_get_texturerect(rect, sprite);

        return Rect {
            x: get_float(rect_class, "x", texturerect),
            y: get_float(rect_class, "y", texturerect),
            width: get_float(rect_class, "width", texturerect),
            height: get_float(rect_class, "height", texturerect),
        };
    }
}

pub fn sprite_to_texture2d(sprite: *mut Il2CppObject) -> *mut Il2CppObject {
    unsafe {
        let sprite_class = get_class_from_image("UnityEngine.CoreModule.dll", "UnityEngine.Sprite");
        let texture2d_class =
            get_class_from_image("UnityEngine.CoreModule.dll", "UnityEngine.Texture2D");

        let texture2d_ctor: unsafe extern "C" fn(*mut Il2CppObject, i32, i32, i32, bool) =
            std::mem::transmute(get_method(texture2d_class, ".ctor", 4));
        let get_pixels: unsafe extern "C" fn(
            *mut Il2CppObject,
            i32,
            i32,
            i32,
            i32,
            i32,
        ) -> *mut Il2CppArray = std::mem::transmute(get_method(texture2d_class, "GetPixels", 5));
        let set_pixels: unsafe extern "C" fn(*mut Il2CppObject, *mut Il2CppArray, i32) =
            std::mem::transmute(get_method(texture2d_class, "SetPixels", 2));

        let texture = get_object(sprite_class, "texture", sprite);
        let newtexture = texture_to_texture2d(texture);
        let rect = rect_from_sprite(sprite);
        let colors = get_pixels(
            newtexture,
            rect.x as i32,
            rect.y as i32,
            rect.width as i32,
            rect.height as i32,
            0,
        );
        let outtexture = object_new(texture2d_class);
        texture2d_ctor(outtexture, rect.width as i32, rect.height as i32, 4, false);
        set_pixels(outtexture, colors, 0);
        return outtexture;
    }
}

pub struct UiManager {
    instance: *mut Il2CppObject,
    load_atlas_addr:
        unsafe extern "C" fn(*mut Il2CppObject, i32, bool, *const MethodInfo) -> *mut Il2CppObject,
}

impl UiManager {
    pub fn init() -> Self {
        unsafe {
            let class = get_gallop_class("UIManager");
            return UiManager {
                instance: get_singleton(class),
                load_atlas_addr: std::mem::transmute(get_method(class, "LoadAtlas", 2)),
            };
        }
    }

    pub fn load_atlas(&self, atlas_type: i32, on_view: bool) -> *mut Il2CppObject {
        unsafe {
            return (self.load_atlas_addr)(self.instance, atlas_type, on_view, null());
        }
    }
}
