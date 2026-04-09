#![feature(mpmc_channel)]

pub mod il2cpp;
pub mod plugin_api;

use il2cpp::types::*;
use int_enum::IntEnum;
use plugin_api::{InitResult, Vtable};
use serde::Serialize;
use std::{
    ffi::{CString, c_char, c_void},
    net::{TcpListener, TcpStream},
    ptr::{null, null_mut},
    sync::{
        LazyLock,
        mpmc::{Receiver, Sender, channel},
    },
    thread,
};
use tungstenite::{Message::Text, Utf8Bytes, accept};

type DialogTrainedCharacterDetail_CreateSetupParameter = unsafe extern "C" fn(
    *const Il2CppObject,
    *const c_char,
    *const c_void,
    bool,
    bool,
    *const c_void,
);
type GetI32 = unsafe extern "C" fn(*const Il2CppObject, *const c_void) -> i32;
type GetPointer = unsafe extern "C" fn(*const Il2CppObject, *const c_void) -> *const c_void;

static mut VTABLE: Option<&'static Vtable> = None;
static TXRX: LazyLock<(Sender<String>, Receiver<String>)> = LazyLock::new(|| channel());
static TX: LazyLock<Sender<String>> = LazyLock::new(|| TXRX.0.clone());
static RX: LazyLock<Receiver<String>> = LazyLock::new(|| TXRX.1.clone());

#[derive(Default, Serialize, IntEnum)]
#[repr(C)]
enum SkillTag {
    #[default]
    RunningStyleBegin = 100,
    Nige = 101,
    Senko = 102,
    Sashi = 103,
    Oikomi = 104,
    RunningStyleEnd = 199,
    DistanceBegin = 200,
    Short = 201,
    Mile = 202,
    Middle = 203,
    Long = 204,
    DistanceEnd = 299,
    SPEED = 401,
    STAMINA = 402,
    POWER = 403,
    GUTS = 404,
    WIZ = 405,
    DOWN = 406,
    SPECIAL = 407,
    GroundBegin = 500,
    Turf = 501,
    Dirt = 502,
    GroundEnd = 599,
    ScenarioBegin = 800,
    ScenarioEnd = 899,
}

#[derive(Default, Serialize)]
struct SkillData {
    name: String,
    remark: String,
    skill_tags: Vec<SkillTag>,
    icon_id: i32,
    is_level_up: bool,
    grade_value: i32,
}

#[derive(Default, Serialize)]
struct CharacterData {
    rank_score: i32,
    speed: i32,
    stamina: i32,
    power: i32,
    guts: i32,
    wiz: i32,
    proper_ground_turf: i32,
    proper_ground_dirt: i32,
    proper_distance_short: i32,
    proper_distance_mile: i32,
    proper_distance_middle: i32,
    proper_distance_long: i32,
    proper_running_style_nige: i32,
    proper_running_style_senko: i32,
    proper_running_style_sashi: i32,
    proper_running_style_oikomi: i32,
    acquired_skills: Vec<SkillData>,
}

fn array_get_obj(array: &Il2CppArray, index: usize) -> *mut Il2CppObject {
    unsafe {
        let data_ptr = (array as *const _ as *const u8).add(32) as *const *mut Il2CppObject;
        *data_ptr.add(index)
    }
}

fn array_get_int(array: &Il2CppArray, index: usize) -> i32 {
    unsafe {
        let data_ptr = (array as *const _ as *const u8).add(32) as *const i32;
        *data_ptr.add(index)
    }
}

fn il2cppstring_as_string(string: &Il2CppString) -> String {
    let slice =
        unsafe { std::slice::from_raw_parts(string.chars.as_ptr(), string.length as usize) };
    return String::from_utf16_lossy(slice);
}

unsafe fn log(log_level: i32, log_str: &str) {
    unsafe {
        (VTABLE.unwrap().log)(
            log_level,
            CString::new("UISC").unwrap().as_ptr() as *const c_char,
            CString::new(log_str).unwrap().as_ptr() as *const c_char,
        );
    }
}

unsafe fn get_hachimi_and_interceptor() -> (*const c_void, *const c_void) {
    unsafe {
        let vtable = VTABLE.unwrap();
        let hachimi = (vtable.hachimi_instance)();
        let interceptor = (vtable.hachimi_get_interceptor)(hachimi);
        (hachimi, interceptor)
    }
}

fn get_i32(class: *mut Il2CppClass, property: &str, this: *const Il2CppObject) -> i32 {
    unsafe {
        let getter_name = CString::new(format!("get_{property}")).unwrap();
        let vtable = VTABLE.unwrap();
        let getter: GetI32 = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            class,
            getter_name.as_ptr(),
            0,
        ));
        return getter(this, null());
    }
}

fn get_pointer(
    class: *mut Il2CppClass,
    property: &str,
    this: *const Il2CppObject,
) -> *const c_void {
    unsafe {
        let getter_name = CString::new(format!("get_{property}")).unwrap();
        let vtable = VTABLE.unwrap();
        let getter: GetPointer = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            class,
            getter_name.as_ptr(),
            0,
        ));
        return getter(this, null());
    }
}

fn get_object(
    class: *mut Il2CppClass,
    property: &str,
    this: *const Il2CppObject,
) -> *const Il2CppObject {
    return get_pointer(class, property, this) as *const Il2CppObject;
}

fn trainedcharadata_to_struct(trained_chara_data: *const Il2CppObject) -> CharacterData {
    unsafe {
        let vtable = VTABLE.unwrap();
        let image = (vtable.il2cpp_get_assembly_image)(c"umamusume".as_ptr());

        let work_trained_chara_data_class =
            (vtable.il2cpp_get_class)(image, c"Gallop".as_ptr(), c"WorkTrainedCharaData".as_ptr());
        let trained_chara_data_class = (vtable.il2cpp_find_nested_class)(
            work_trained_chara_data_class,
            c"TrainedCharaData".as_ptr(),
        );
        let work_skill_data_class =
            (vtable.il2cpp_get_class)(image, c"Gallop".as_ptr(), c"WorkSkillData".as_ptr());
        let acquired_skill_class =
            (vtable.il2cpp_find_nested_class)(work_skill_data_class, c"AcquiredSkill".as_ptr());
        let master_skill_data_class =
            (vtable.il2cpp_get_class)(image, c"Gallop".as_ptr(), c"MasterSkillData".as_ptr());
        let skill_data_class =
            (vtable.il2cpp_find_nested_class)(master_skill_data_class, c"SkillData".as_ptr());

        let getter_i32 = |property: &str| -> i32 {
            return get_i32(trained_chara_data_class, property, trained_chara_data);
        };
        let get_enum_tag_list: GetPointer = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            skill_data_class,
            c"GetEnumTagList".as_ptr(),
            0,
        ));

        let grade_value_field =
            (vtable.il2cpp_get_field_from_name)(skill_data_class, c"GradeValue".as_ptr());

        let skill_list_il2cpp = get_pointer(
            trained_chara_data_class,
            "AcquiredSkillArray",
            trained_chara_data,
        ) as *const Il2CppArray;
        let mut skill_vec: Vec<SkillData> = Vec::new();

        for i in 0..(*skill_list_il2cpp).max_length {
            let skill = array_get_obj(skill_list_il2cpp.as_ref_unchecked(), i);
            let skill_data = get_object(acquired_skill_class, "MasterData", skill);
            let name = get_object(skill_data_class, "Name", skill_data) as *const Il2CppString;
            let name_string = il2cppstring_as_string(&(*name));
            let remark = get_object(skill_data_class, "Remarks", skill_data) as *const Il2CppString;
            let remark_string = il2cppstring_as_string(&(*remark));

            let skill_tag_list = get_enum_tag_list(skill_data, null()) as *mut Il2CppObject;
            let list_class = *(skill_tag_list as *mut *mut Il2CppClass);
            let size_field = (vtable.il2cpp_get_field_from_name)(list_class, c"_size".as_ptr());
            let item_field = (vtable.il2cpp_get_field_from_name)(list_class, c"_items".as_ptr());
            let mut size: i32 = 0;
            (vtable.il2cpp_get_field_value)(skill_tag_list, size_field, &mut size as *mut _ as _);
            let mut skill_tag_array: *mut Il2CppArray = null_mut();
            (vtable.il2cpp_get_field_value)(
                skill_tag_list,
                item_field,
                &mut skill_tag_array as *mut _ as _,
            );
            let mut skill_tag_vec: Vec<SkillTag> = Vec::new();
            for j in 0..size as usize {
                let skill_tag_int = array_get_int(skill_tag_array.as_ref_unchecked(), j);
                if let Ok(skill_tag) = SkillTag::try_from(skill_tag_int as isize) {
                    skill_tag_vec.push(skill_tag);
                }
            }

            let mut grade_value = 0;
            (vtable.il2cpp_get_field_value)(
                skill_data as *mut Il2CppObject,
                grade_value_field,
                &mut grade_value as *mut _ as _,
            );

            skill_vec.push(SkillData {
                name: name_string,
                remark: remark_string,
                skill_tags: skill_tag_vec,
                icon_id: 0,
                is_level_up: false,
                grade_value: grade_value,
            });
        }

        return CharacterData {
            rank_score: getter_i32("RankScore"),
            speed: getter_i32("Speed"),
            stamina: getter_i32("Stamina"),
            power: getter_i32("Power"),
            guts: getter_i32("Guts"),
            wiz: getter_i32("Wiz"),
            proper_ground_turf: getter_i32("ProperGroundTurf"),
            proper_ground_dirt: getter_i32("ProperGroundDirt"),
            proper_distance_short: getter_i32("ProperDistanceShort"),
            proper_distance_mile: getter_i32("ProperDistanceMile"),
            proper_distance_middle: getter_i32("ProperDistanceMiddle"),
            proper_distance_long: getter_i32("ProperDistanceLong"),
            proper_running_style_nige: getter_i32("ProperRunningStyleNige"),
            proper_running_style_senko: getter_i32("ProperRunningStyleSenko"),
            proper_running_style_sashi: getter_i32("ProperRunningStyleSashi"),
            proper_running_style_oikomi: getter_i32("ProperRunningStyleOikomi"),
            acquired_skills: skill_vec,
        };
    }
}

unsafe extern "C" fn DialogTrainedCharacterDetail_CreateSetupParameter_hook(
    trained_chara_data: *const Il2CppObject,
    trainer_name: *const c_char,
    on_change_partner: *const c_void,
    is_single_mode: bool,
    is_follow: bool,
    method_info: *const c_void,
) {
    unsafe {
        let vtable = VTABLE.unwrap();
        let tx = TX.clone();

        log(0, "CreateSetupParameter called");

        let (_, interceptor) = get_hachimi_and_interceptor();
        let trampoline = (vtable.interceptor_get_trampoline_addr)(
            interceptor,
            DialogTrainedCharacterDetail_CreateSetupParameter_hook as *mut c_void,
        );
        let original: DialogTrainedCharacterDetail_CreateSetupParameter =
            std::mem::transmute(trampoline);

        let character_data = trainedcharadata_to_struct(trained_chara_data);
        tx.send(serde_json::to_string(&character_data).unwrap())
            .unwrap();

        drop(character_data);
        drop(tx);

        log(0, "Calling original function");
        original(
            trained_chara_data,
            trainer_name,
            on_change_partner,
            is_single_mode,
            is_follow,
            method_info,
        );
    }
}

fn websocket_handler(stream: TcpStream) {
    let mut ws = accept(stream).unwrap();
    let rx = RX.clone();
    for msg in rx.iter() {
        ws.send(Text(Utf8Bytes::from(msg))).unwrap();
    }
}

#[unsafe(export_name = "hachimi_init")]
pub extern "C" fn hachimi_init(vtable: *const Vtable, version: i32) -> InitResult {
    if vtable.is_null() {
        return InitResult::Error;
    }
    if version < 2 {
        return InitResult::Error;
    }

    unsafe {
        VTABLE = Some(&*vtable);
        let vtable = VTABLE.unwrap();

        log(0, "Hooking started");
        let (_, interceptor) = get_hachimi_and_interceptor();

        let image = (vtable.il2cpp_get_assembly_image)(c"umamusume".as_ptr());
        let klass = (vtable.il2cpp_get_class)(
            image,
            c"Gallop".as_ptr(),
            c"DialogTrainedCharacterDetail".as_ptr(),
        );
        let createsetupparameter_addr =
            (vtable.il2cpp_get_method_addr)(klass, c"CreateSetupParameter".as_ptr(), 5);
        (vtable.interceptor_hook)(
            interceptor,
            createsetupparameter_addr,
            DialogTrainedCharacterDetail_CreateSetupParameter_hook as *mut c_void,
        );

        log(0, "Hooking finished");

        log(0, "Spawning server thread");
        thread::spawn(|| {
            log(0, "Starting server thread");
            let addr = "127.0.0.1:5555";
            let server = TcpListener::bind(addr).unwrap();
            log(0, format!("Listening on {addr}").as_str());

            for stream in server.incoming() {
                log(0, "Got connection");
                thread::spawn(|| websocket_handler(stream.unwrap()));
            }
        });
    }

    InitResult::Ok
}
