extends Control

var character_data: Variant
var counter: int = 0
var host: Variant = JavaScriptBridge.eval("window.location.host;")
var list_items: Dictionary[int, SkillListItem] = { }
var potential_skills: Dictionary[int, Variant] = { }
var score: int = 0:
    set(value):
        %ScoreLabel.text = format_number_with_commas(int(value))
        score = int(value)
        %HTTPRequest.request_completed.connect(on_request_completed_get_rank)
        %HTTPRequest.request("http://%s/rank_icon/%d" % [host, score])
var score_calc: Score = Score.new()
var ws: WebSocketPeer = WebSocketPeer.new()


func format_number_with_commas(number: int) -> String:
    var num_str: String = str(abs(number))
    var result: String = ""
    var count: int = 0
    var sep: String = ","

    for i in range(num_str.length() - 1, -1, -1):
        result = num_str[i] + result
        count += 1
        if count % 3 == 0 and i != 0:
            result = sep + result

    if number < 0:
        result = "-" + result

    return result


func on_request_completed_get_ws(_result: int, _response_code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
    var port: String = body.get_string_from_ascii()
    ws.connect_to_url("ws://127.0.0.1:%s" % port)
    %HTTPRequest.request_completed.disconnect(on_request_completed_get_ws)


func on_request_completed_get_rank(_result: int, _response_code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
    var image: Image = Image.new()
    image.load_png_from_buffer(body)
    %RankIcon.texture = ImageTexture.create_from_image(image)
    %HTTPRequest.request_completed.disconnect(on_request_completed_get_rank)


func update_chara_data(chara_data: Variant) -> void:
    character_data = chara_data
    potential_skills.clear()
    list_items.clear()
    for child in %SkillsContainer.get_children():
        %SkillsContainer.remove_child(child)
    %ParamBar.speed_value = chara_data.speed
    %ParamBar.stamina_value = chara_data.stamina
    %ParamBar.power_value = chara_data.power
    %ParamBar.guts_value = chara_data.guts
    %ParamBar.wiz_value = chara_data.wiz
    score = score_calc.calculate_score(chara_data)
    for skill: Variant in chara_data.acquired_skills:
        potential_skills[skill.group_id] = skill
        add_skill_item(skill)


func add_skill_item(skill: Variant) -> SkillListItem:
    var list_item: SkillListItem = preload("res://scenes/skill_list_item.tscn").instantiate()
    list_item.setup(skill)
    %SkillsContainer.add_child(list_item, true)
    list_items[skill.group_id] = list_item
    return list_item


func add_potential_skill(skills: Variant) -> void:
    var skill_data: Variant = skills[0]
    var group_id: int = skill_data.group_id
    var skill_score: int = score_calc.calculate_adjusted_skill_score(skill_data, character_data)
    skill_data.grade_value = skill_score
    var list_item: SkillListItem = preload("res://scenes/skill_list_item.tscn").instantiate()
    list_item.setup(skill_data)
    if group_id in potential_skills:
        var prev_skill: Variant = potential_skills[group_id]
        potential_skills[group_id] = skill_data
        score = score + skill_score - prev_skill.grade_value
        %SkillsContainer.remove_child(list_items[group_id])
        list_items[group_id] = list_item
        %SkillsContainer.add_child(list_item, true)
    else:
        potential_skills[group_id] = skill_data
        score = score + skill_score
        list_items[group_id] = list_item
        %SkillsContainer.add_child(list_item, true)


func remove_potential_skill(skills: Variant) -> void:
    var group_id: int = skills[0].group_id
    if group_id in potential_skills:
        var prev_skill: Variant = potential_skills[group_id]
        if len(skills) > 1 and int(prev_skill.id) % 10 == 1:
            add_potential_skill([skills[0]])
            return
        score = score - prev_skill.grade_value
        %SkillsContainer.remove_child(list_items[group_id])
        potential_skills.erase(group_id)
        list_items.erase(group_id)


func _process(_delta: float) -> void:
    ws.poll()

    var state: int = ws.get_ready_state()

    if state == WebSocketPeer.STATE_OPEN:
        while ws.get_available_packet_count():
            var packet: PackedByteArray = ws.get_packet()
            if ws.was_string_packet():
                counter += 1
                var message_string: String = packet.get_string_from_utf8()
                var message: Variant = JSON.parse_string(message_string)
                var eval_command: String = "console.log(JSON.parse('%s'))" % message_string.json_escape()
                JavaScriptBridge.eval(eval_command)
                match message.message_type:
                    "CharacterUpdate":
                        update_chara_data(message.message.CharacterUpdate)
                    "SkillPlus":
                        add_potential_skill(message.message.SkillUpdate)
                    "SkillMinus":
                        remove_potential_skill(message.message.SkillUpdate)
            else:
                ws.send(packet, WebSocketPeer.WRITE_MODE_BINARY)


func _ready() -> void:
    if not host:
        host = "localhost:5555"
    %HTTPRequest.request_completed.connect(on_request_completed_get_ws)
    %HTTPRequest.request("http://%s/socket" % host)
