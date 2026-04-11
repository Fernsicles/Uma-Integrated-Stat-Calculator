@tool
extends VBoxContainer

@export var param_name: String = "":
    set(value):
        param_name = value
        %ParamName.text = value
@export var param_value: int = 0:
    set(value):
        param_value = value
        %ParamValue.text = str(value)


func _ready() -> void:
    %ParamName.text = param_name
    %ParamValue.text = str(param_value)
