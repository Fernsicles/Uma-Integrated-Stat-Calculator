class_name SkillListItem
extends PanelContainer

func setup(skill_data: Variant) -> void:
	self.name = str(skill_data.id / 10)
	%Name.text = skill_data.name
	%Remark.text = skill_data.remark
	%Score.text = str(int(skill_data.grade_value))
