extends HBoxContainer

@export var speed_value: int = 0:
    set(value):
        speed_value = value
        %SpeedBox.param_value = value
@export var stamina_value: int = 0:
    set(value):
        stamina_value = value
        %StamBox.param_value = value
@export var power_value: int = 0:
    set(value):
        power_value = value
        %PowBox.param_value = value
@export var guts_value: int = 0:
    set(value):
        guts_value = value
        %GutsBox.param_value = value
@export var wiz_value: int = 0:
    set(value):
        wiz_value = value
        %WizBox.param_value = value
