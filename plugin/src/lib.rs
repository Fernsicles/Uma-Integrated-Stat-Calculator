#![feature(mpmc_channel)]

pub mod il2cpp;
pub mod plugin_api;

use bytes::Buf;
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
use tungstenite::{
    Bytes,
    Message::{Binary, Text},
    Utf8Bytes, accept,
};

use crate::plugin_api::VERSION;

type DialogTrainedCharacterDetail_CreateSetupParameter =
    unsafe extern "C" fn(*mut Il2CppObject, *mut c_char, *mut c_void, bool, bool, *mut MethodInfo);
type OnClickListItem = unsafe extern "C" fn(*mut Il2CppObject, *mut Il2CppObject, *mut MethodInfo);
type GetU8 = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> u8;
type GetI32 = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> i32;
type GetPointer = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> *mut c_void;
type GetObject = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> *mut Il2CppObject;

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
    id: i32,
    name: String,
    rarity: i32,
    level: i32,
    remark: String,
    skill_tags: Vec<SkillTag>,
    grade_value: i32,
    is_level_up: bool,
    is_unique_skill: bool,
}

#[derive(Default, Serialize)]
struct CharacterData {
    id: i32,
    name: String,
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

#[derive(Serialize)]
enum MessageType {
    CharacterUpdate,
    SkillPlus,
    SkillMinus,
}

#[derive(Serialize)]
enum MessageData {
    CharacterUpdate(CharacterData),
    SkillUpdate(SkillData),
}

#[derive(Serialize)]
struct Message {
    message_type: MessageType,
    message: MessageData,
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

fn get_class(path: &str) -> *mut Il2CppClass {
    let path_parts = path.split('.');
    let mut path_parts: Vec<&str> = path_parts.collect();
    unsafe {
        let vtable = VTABLE.unwrap();
        let image = (vtable.il2cpp_get_assembly_image)(c"umamusume".as_ptr());
        let namespace = path_parts.remove(0);
        let mut class = (vtable.il2cpp_get_class)(
            image,
            CString::new(namespace).unwrap().as_ptr(),
            CString::new(path_parts.remove(0)).unwrap().as_ptr(),
        );
        for part in path_parts.iter() {
            class = (vtable.il2cpp_find_nested_class)(class, CString::new(*part).unwrap().as_ptr());
        }
        return class;
    }
}

fn get_gallop_class(class_name: &str) -> *mut Il2CppClass {
    return get_class(("Gallop.".to_owned() + class_name).as_str());
}

fn get_i32(class: *mut Il2CppClass, property: &str, this: *mut Il2CppObject) -> i32 {
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

fn get_u8(class: *mut Il2CppClass, property: &str, this: *mut Il2CppObject) -> u8 {
    unsafe {
        let getter_name = CString::new(format!("get_{property}")).unwrap();
        let vtable = VTABLE.unwrap();
        let getter: GetU8 = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            class,
            getter_name.as_ptr(),
            0,
        ));
        return getter(this, null());
    }
}

fn get_i32_field(class: *mut Il2CppClass, field: &str, this: *const Il2CppObject) -> i32 {
    unsafe {
        let vtable = VTABLE.unwrap();
        let field =
            (vtable.il2cpp_get_field_from_name)(class, CString::new(field).unwrap().as_ptr());
        let mut value = 0;
        (vtable.il2cpp_get_field_value)(
            this as *mut Il2CppObject,
            field,
            &mut value as *mut _ as _,
        );
        return value;
    }
}

fn get_pointer(class: *mut Il2CppClass, property: &str, this: *mut Il2CppObject) -> *const c_void {
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
    this: *mut Il2CppObject,
) -> *mut Il2CppObject {
    return get_pointer(class, property, this) as *mut Il2CppObject;
}

fn chara_id_to_icon(id: i32) {
    unsafe {
        let vtable = VTABLE.unwrap();
        let character_button_info_class = get_gallop_class("CharacterButtonInfo");
        let character_button_class = get_gallop_class("CharacterButton");

        type Ctor1 = unsafe extern "C" fn(
            *mut Il2CppObject,
            id: i32,
            id_type: i32,
            method_info: *const c_void,
        );
        let character_button_info_constructor: Ctor1 = std::mem::transmute((vtable
            .il2cpp_get_method_addr)(
            character_button_info_class,
            c".ctor".as_ptr(),
            2,
        ));

        type Ctor = unsafe extern "C" fn(*mut Il2CppObject);
        let character_button_constructor: Ctor = std::mem::transmute((vtable
            .il2cpp_get_method_addr)(
            character_button_class,
            c".ctor".as_ptr(),
            0,
        ));

        let character_button_info = (vtable.il2cpp_object_new)(character_button_info_class);
        character_button_info_constructor(character_button_info, id, 0, null());
        let character_button = (vtable.il2cpp_object_new)(character_button_class);
        character_button_constructor(character_button);
    }
}

fn skilldata_to_struct(skill_data: *mut Il2CppObject) -> SkillData {
    unsafe {
        let vtable = VTABLE.unwrap();

        let skill_data_class = get_class("Gallop.MasterSkillData.SkillData");

        let get_enum_tag_list: GetPointer = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            skill_data_class,
            c"GetEnumTagList".as_ptr(),
            0,
        ));
        let is_unique_skill: GetU8 = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            skill_data_class,
            c"IsUniqueSkill".as_ptr(),
            0,
        ));

        let name = get_object(skill_data_class, "Name", skill_data) as *const Il2CppString;
        let name_string = il2cppstring_as_string(name.as_ref_unchecked());
        let remark = get_object(skill_data_class, "Remarks", skill_data) as *const Il2CppString;
        let remark_string = il2cppstring_as_string(remark.as_ref_unchecked());

        let skill_tag_list = get_enum_tag_list(skill_data, null()) as *mut Il2CppObject;
        let list_class = *(skill_tag_list as *mut *mut Il2CppClass);
        let item_field = (vtable.il2cpp_get_field_from_name)(list_class, c"_items".as_ptr());
        let size = get_i32_field(list_class, "_size", skill_tag_list);
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

        let getter_i32_field =
            |field: &str| -> i32 { return get_i32_field(skill_data_class, field, skill_data) };

        return SkillData {
            id: getter_i32_field("Id"),
            name: name_string,
            rarity: getter_i32_field("Rarity"),
            level: 0,
            remark: remark_string,
            skill_tags: skill_tag_vec,
            grade_value: getter_i32_field("GradeValue"),
            is_level_up: get_u8(skill_data_class, "IsLevelUp", skill_data) != 0,
            is_unique_skill: is_unique_skill(skill_data, null()) != 0,
        };
    }
}

fn acquiredskill_to_struct(acquired_skill: *mut Il2CppObject) -> SkillData {
    let acquired_skill_class = get_class("Gallop.WorkSkillData.AcquiredSkill");

    let level = get_i32(acquired_skill_class, "Level", acquired_skill);
    let skill_data = get_object(acquired_skill_class, "MasterData", acquired_skill);
    let mut output = skilldata_to_struct(skill_data);
    output.level = level;

    return output;
}

fn worksinglemodecharadata_to_struct(
    work_single_mode_chara_data: *mut Il2CppObject,
) -> CharacterData {
    unsafe {
        let vtable = VTABLE.unwrap();

        let work_single_mode_chara_data_class = get_gallop_class("WorkSingleModeCharaData");
        let card_data_class = get_class("Gallop.MasterCardData.CardData");

        let getter_i32 = |property: &str| -> i32 {
            return get_i32(
                work_single_mode_chara_data_class,
                property,
                work_single_mode_chara_data,
            );
        };

        let skill_list = get_object(
            work_single_mode_chara_data_class,
            "AcquiredSkillList",
            work_single_mode_chara_data,
        );

        let list_class = *(skill_list as *mut *mut Il2CppClass);
        let item_field = (vtable.il2cpp_get_field_from_name)(list_class, c"_items".as_ptr());
        let size = get_i32_field(list_class, "_size", skill_list);
        let mut skill_array: *mut Il2CppArray = null_mut();
        (vtable.il2cpp_get_field_value)(skill_list, item_field, &mut skill_array as *mut _ as _);
        let mut skill_vec: Vec<SkillData> = Vec::new();
        for i in 0..size as usize {
            let skill = array_get_obj(skill_array.as_ref_unchecked(), i);
            skill_vec.push(acquiredskill_to_struct(skill));
        }

        let card_data = get_object(
            work_single_mode_chara_data_class,
            "CardData",
            work_single_mode_chara_data,
        );
        let name = get_pointer(card_data_class, "Charaname", card_data) as *mut Il2CppString;

        return CharacterData {
            id: getter_i32("CharaId"),
            name: il2cppstring_as_string(name.as_ref_unchecked()),
            rank_score: 0,
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

fn trainedcharadata_to_struct(trained_chara_data: *mut Il2CppObject) -> CharacterData {
    unsafe {
        let trained_chara_data_class = get_class("Gallop.WorkTrainedCharaData.TrainedCharaData");
        let chara_data_class = get_class("Gallop.MasterCharaData.CharaData");

        let getter_i32 = |property: &str| -> i32 {
            return get_i32(trained_chara_data_class, property, trained_chara_data);
        };

        let skill_list_il2cpp = get_pointer(
            trained_chara_data_class,
            "AcquiredSkillArray",
            trained_chara_data,
        ) as *const Il2CppArray;

        let mut skill_vec: Vec<SkillData> = Vec::new();
        for i in 0..(*skill_list_il2cpp).max_length {
            let skill = array_get_obj(skill_list_il2cpp.as_ref_unchecked(), i);

            skill_vec.push(acquiredskill_to_struct(skill));
        }

        let master_chara_data = get_object(
            trained_chara_data_class,
            "MasterCharaData",
            trained_chara_data,
        );

        let name =
            get_object(trained_chara_data_class, "Name", trained_chara_data) as *const Il2CppString;
        let name_string = il2cppstring_as_string(name.as_ref_unchecked());

        chara_id_to_icon(0);

        return CharacterData {
            id: get_i32_field(chara_data_class, "Id", master_chara_data),
            name: name_string,
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
    trained_chara_data: *mut Il2CppObject,
    trainer_name: *mut c_char,
    on_change_partner: *mut c_void,
    is_single_mode: bool,
    is_follow: bool,
    method_info: *mut MethodInfo,
) {
    unsafe {
        let vtable = VTABLE.unwrap();
        let tx = TX.clone();

        let (_, interceptor) = get_hachimi_and_interceptor();
        let trampoline = (vtable.interceptor_get_trampoline_addr)(
            interceptor,
            DialogTrainedCharacterDetail_CreateSetupParameter_hook as *mut c_void,
        );
        let original: DialogTrainedCharacterDetail_CreateSetupParameter =
            std::mem::transmute(trampoline);

        let character_data = trainedcharadata_to_struct(trained_chara_data);
        let message = Message {
            message_type: MessageType::CharacterUpdate,
            message: MessageData::CharacterUpdate(character_data),
        };
        tx.send(serde_json::to_string(&message).unwrap()).unwrap();

        drop(message);
        drop(tx);

        return original(
            trained_chara_data,
            trainer_name,
            on_change_partner,
            is_single_mode,
            is_follow,
            method_info,
        );
    }
}

unsafe extern "C" fn PartsSingleModeCharacterStatusPanel_Setup_hook(
    this: *mut Il2CppObject,
    chara_data: *mut Il2CppObject,
    method_info: *mut MethodInfo,
) {
    unsafe {
        let vtable = VTABLE.unwrap();
        let tx = TX.clone();

        let (_, interceptor) = get_hachimi_and_interceptor();
        let trampoline = (vtable.interceptor_get_trampoline_addr)(
            interceptor,
            PartsSingleModeCharacterStatusPanel_Setup_hook as *mut c_void,
        );
        let original: OnClickListItem = std::mem::transmute(trampoline);

        let character_data = worksinglemodecharadata_to_struct(chara_data);
        let message = Message {
            message_type: MessageType::CharacterUpdate,
            message: MessageData::CharacterUpdate(character_data),
        };
        tx.send(serde_json::to_string(&message).unwrap()).unwrap();

        drop(message);
        drop(tx);

        return original(this, chara_data, method_info);
    }
}

fn on_click_list_item_common(item: *mut Il2CppObject) -> SkillData {
    unsafe {
        let vtable = VTABLE.unwrap();

        let parts_single_mode_skill_learning_list_item_class =
            get_gallop_class("PartsSingleModeSkillLearningListItem");
        let info_class = get_class("Gallop.PartsSingleModeSkillLearningListItem.Info");

        let get_top_info: GetObject = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            parts_single_mode_skill_learning_list_item_class,
            c"GetTopInfo".as_ptr(),
            0,
        ));

        let info = get_top_info(item, null());
        let master_data = get_object(info_class, "MasterData", info);
        let mut skill_data = skilldata_to_struct(master_data);
        skill_data.level = get_i32(info_class, "Level", info);

        return skill_data;
    }
}

unsafe extern "C" fn SingleModeSkillLearningViewController_OnClickPlusListItem_hook(
    this: *mut Il2CppObject,
    item: *mut Il2CppObject,
    method_info: *mut MethodInfo,
) {
    unsafe {
        let vtable = VTABLE.unwrap();
        let tx = TX.clone();

        let (_, interceptor) = get_hachimi_and_interceptor();
        let trampoline = (vtable.interceptor_get_trampoline_addr)(
            interceptor,
            SingleModeSkillLearningViewController_OnClickPlusListItem_hook as *mut c_void,
        );
        let original: OnClickListItem = std::mem::transmute(trampoline);

        let skill_data = on_click_list_item_common(item);

        let message = Message {
            message_type: MessageType::SkillPlus,
            message: MessageData::SkillUpdate(skill_data),
        };
        tx.send(serde_json::to_string(&message).unwrap()).unwrap();

        drop(message);
        drop(tx);

        return original(this, item, method_info);
    }
}

unsafe extern "C" fn SingleModeSkillLearningViewController_OnClickMinusListItem_hook(
    this: *mut Il2CppObject,
    item: *mut Il2CppObject,
    method_info: *mut MethodInfo,
) {
    unsafe {
        let vtable = VTABLE.unwrap();
        let tx = TX.clone();

        let (_, interceptor) = get_hachimi_and_interceptor();
        let trampoline = (vtable.interceptor_get_trampoline_addr)(
            interceptor,
            SingleModeSkillLearningViewController_OnClickMinusListItem_hook as *mut c_void,
        );
        let original: OnClickListItem = std::mem::transmute(trampoline);

        let skill_data = on_click_list_item_common(item);

        let message = Message {
            message_type: MessageType::SkillMinus,
            message: MessageData::SkillUpdate(skill_data),
        };
        tx.send(serde_json::to_string(&message).unwrap()).unwrap();

        drop(message);
        drop(tx);

        return original(this, item, method_info);
    }
}

fn websocket_handler(stream: TcpStream) {
    let mut ws = accept(stream).unwrap();
    let rx = RX.clone();
    let tx = TX.clone();
    let mut counter: u32 = 0;
    for msg in rx.iter() {
        counter += 1;
        ws.send(Binary(Bytes::copy_from_slice(&counter.to_le_bytes())))
            .unwrap();
        if let Ok(response) = ws.read() {
            if response.into_data().get_u32_le() != counter {
                unsafe {
                    log(0, format!("Socket closed").as_str());
                }
                tx.send(msg).unwrap();
                return;
            }
        } else {
            unsafe {
                log(0, format!("Socket closed").as_str());
            }
            tx.send(msg).unwrap();
            return;
        }

        unsafe {
            log(0, format!("Sending message: {counter}").as_str());
        }
        ws.send(Text(Utf8Bytes::from(msg))).unwrap();
    }
}

#[unsafe(export_name = "hachimi_init")]
pub extern "C" fn hachimi_init(vtable: *const Vtable, version: i32) -> InitResult {
    if vtable.is_null() {
        return InitResult::Error;
    }
    if version < VERSION {
        return InitResult::Error;
    }

    unsafe {
        VTABLE = Some(&*vtable);
        let vtable = VTABLE.unwrap();

        log(0, "Hooking started");
        let (_, interceptor) = get_hachimi_and_interceptor();

        let createsetupparameter_addr = (vtable.il2cpp_get_method_addr)(
            get_gallop_class("DialogTrainedCharacterDetail"),
            c"CreateSetupParameter".as_ptr(),
            5,
        );
        (vtable.interceptor_hook)(
            interceptor,
            createsetupparameter_addr,
            DialogTrainedCharacterDetail_CreateSetupParameter_hook as *mut c_void,
        );

        let setup_addr = (vtable.il2cpp_get_method_addr)(
            get_gallop_class("PartsSingleModeCharacterStatusPanel"),
            c"Setup".as_ptr(),
            1,
        );
        (vtable.interceptor_hook)(
            interceptor,
            setup_addr,
            PartsSingleModeCharacterStatusPanel_Setup_hook as *mut c_void,
        );

        let on_click_plus = (vtable.il2cpp_get_method_addr)(
            get_gallop_class("SingleModeSkillLearningViewController"),
            c"OnClickPlusListItem".as_ptr(),
            1,
        );
        (vtable.interceptor_hook)(
            interceptor,
            on_click_plus,
            SingleModeSkillLearningViewController_OnClickPlusListItem_hook as *mut c_void,
        );

        let on_click_minus = (vtable.il2cpp_get_method_addr)(
            get_gallop_class("SingleModeSkillLearningViewController"),
            c"OnClickMinusListItem".as_ptr(),
            1,
        );
        (vtable.interceptor_hook)(
            interceptor,
            on_click_minus,
            SingleModeSkillLearningViewController_OnClickMinusListItem_hook as *mut c_void,
        );

        log(0, "Hooking finished");

        log(0, "Spawning websocket server thread");
        let ws_server = TcpListener::bind("127.0.0.1:0").unwrap();
        let ws_port = ws_server.local_addr().unwrap().port();
        thread::spawn(move || {
            log(0, "Starting websocket server thread");

            for stream in ws_server.incoming() {
                log(0, "Got connection");
                thread::spawn(|| websocket_handler(stream.unwrap()));
            }
        });

        log(0, "Spawning web server thread");
        thread::spawn(move || {
            log(0, "Starting web server thread");

            rouille::start_server("127.0.0.1:5555", move |request| {
                rouille::router!(request,
                    (GET) (/socket) => {
                        rouille::Response::text(ws_port.to_string())
                    },
                    _ => {
                        rouille::match_assets(&request, "uisc")
                    }
                )
            });
        });
    }

    InitResult::Ok
}
