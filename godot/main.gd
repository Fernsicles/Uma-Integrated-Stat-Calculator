extends Control

var counter: int = 0
var host: Variant = JavaScriptBridge.eval("window.location.host;")
var score_calc: Score = Score.new()
var ws: WebSocketPeer = WebSocketPeer.new()


func on_request_completed(_result: int, _response_code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
    var port: String = body.get_string_from_ascii()
    ws.connect_to_url("ws://127.0.0.1:%s" % port)


func update_chara_data(chara_data: Variant) -> void:
    %SpeedBox.param_value = chara_data.speed
    %StamBox.param_value = chara_data.stamina
    %PowBox.param_value = chara_data.power
    %GutsBox.param_value = chara_data.guts
    %WizBox.param_value = chara_data.wiz
    %ScoreLabel.text = str(score_calc.calculate_score(chara_data))


func _process(_delta: float) -> void:
    ws.poll()

    var state: int = ws.get_ready_state()

    if state == WebSocketPeer.STATE_OPEN:
        while ws.get_available_packet_count():
            var packet: PackedByteArray = ws.get_packet()
            if ws.was_string_packet():
                counter += 1
                var chara_data_string: String = packet.get_string_from_utf8()
                var chara_data: Variant = JSON.parse_string(chara_data_string)
                var eval_command: String = "console.log(JSON.parse('%s'))" % chara_data_string.json_escape()
                JavaScriptBridge.eval(eval_command)
                update_chara_data(chara_data)
            else:
                ws.send(packet, WebSocketPeer.WRITE_MODE_BINARY)


func _ready() -> void:
    %HTTPRequest.request_completed.connect(on_request_completed)
    %HTTPRequest.request("http://%s/socket" % host)
