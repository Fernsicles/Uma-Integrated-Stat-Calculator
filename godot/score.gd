class_name Score
extends Object

const MAX_STATE_VALUE = 2500
const RUNNING_STYLE_TAGS = [
	"Nige",
	"Senko",
	"Sashi",
	"Oikomi",
]
const DISTANCE_TAGS = [
	"Short",
	"Mile",
	"Middle",
	"Long",
]
const APTITUDE_TO_MULTIPLIER = {
	8: 1.1, # S
	7: 1.1, # A
	6: 0.9, # B
	5: 0.9, # C
	4: 0.8, # D
	3: 0.8, # E
	2: 0.8, # F
	1: 0.7, # G
	0: 1.0, # None
}

var score_table: Array[int]


# Taken from https://github.com/daftuyda/UmaTools/blob/main/js/rating-shared.js
# which is taken from https://github.com/umakonga-t/hyokatenCalc/blob/main/main.js
func gen_score_table() -> Array[int]:
	var R1: Array[int] = [5, 8, 10, 13, 16, 18, 21, 24, 26, 28, 29, 30, 31, 33, 34, 35, 39, 41, 42, 43, 52, 55, 66, 68, 68]
	var R2: Array[int] = [79, 80, 81, 83, 84, 85, 86, 88, 89, 90, 92, 93, 94, 96, 97, 98, 100, 101, 102, 103, 105, 106, 107, 109, 110, 111, 113, 114, 115, 117, 118, 119, 121, 122, 123, 124, 126, 127, 128, 130, 131, 132, 134, 135, 136, 138, 139, 140, 141, 143, 144, 145, 147, 148, 149, 151, 152, 153, 155, 156, 157, 159, 160, 161, 162, 164, 165, 166, 168, 169, 170, 172, 173, 174, 176, 177, 178, 179, 181, 182, 182]
	var sc: Array[int] = [0]
	var raw: int = 0
	var idx: int = 0
	for c in range(1, 1201):
		if c <= 49:
			idx = 0
		elif c <= 99:
			idx = 1
		elif c % 50 == 0:
			idx += 1
		raw += R1[idx]
		sc.insert(c, int(round(raw / 10.0)))
	raw = 38413
	idx = 0
	for c in range(1201, 2001):
		if c <= 1209:
			idx = 0
		elif c <= 1219:
			idx = 1
		elif c % 10 == 0:
			idx += 1
		raw += R2[idx]
		sc.insert(c, int(round(raw / 10.0)))
	raw = 142796
	idx = 0
	var rate: int = 183
	for c in range(2001, MAX_STATE_VALUE + 1):
		if idx >= 25:
			rate += 1
			idx = 0
		raw += rate
		idx += 1
		sc.insert(c, int(round(raw / 10.0)))
	return sc


func calculate_stat_score(value: int) -> int:
	return score_table[value]


func calculate_raw_skill_score(skill: Variant) -> int:
	if skill.is_unique_skill:
		return (skill.grade_value / 2) * skill.level
	else:
		return skill.grade_value


func calculate_adjusted_skill_score(skill: Variant, trained_character_data: Variant) -> int:
	var raw_score: int = calculate_raw_skill_score(skill)
	if skill.is_unique_skill:
		return raw_score
	else:
		var score: int = raw_score
		var style_aptitudes: Array[int] = [0]
		var distance_aptitudes: Array[int] = [0]
		for tag: String in skill.skill_tags:
			if tag in RUNNING_STYLE_TAGS:
				var aptitude: int = 0
				match tag:
					"Nige":
						aptitude = trained_character_data.proper_running_style_nige
					"Senko":
						aptitude = trained_character_data.proper_running_style_senko
					"Sashi":
						aptitude = trained_character_data.proper_running_style_sashi
					"Oikomi":
						aptitude = trained_character_data.proper_running_style_oikomi
				style_aptitudes.append(aptitude)
			elif tag in DISTANCE_TAGS:
				var aptitude: int = 0
				match tag:
					"Short":
						aptitude = trained_character_data.proper_distance_short
					"Mile":
						aptitude = trained_character_data.proper_distance_mile
					"Middle":
						aptitude = trained_character_data.proper_distance_middle
					"Long":
						aptitude = trained_character_data.proper_distance_long
				distance_aptitudes.append(aptitude)
		var multipliers: Array[float] = [APTITUDE_TO_MULTIPLIER[style_aptitudes.max()], APTITUDE_TO_MULTIPLIER[distance_aptitudes.max()]]
		return round(score * (multipliers.reduce(func(accum: float, num: float) -> float: return accum * num)))


func calculate_skill_scores(trained_character_data: Variant) -> Dictionary[Variant, int]:
	var skill_scores: Dictionary[Variant, int] = { }
	for skill: Variant in trained_character_data.acquired_skills:
		skill_scores[skill] = calculate_adjusted_skill_score(skill, trained_character_data)
	return skill_scores


func calculate_score(trained_character_data: Variant) -> int:
	var score: int = 0
	score += calculate_stat_score(trained_character_data.speed)
	score += calculate_stat_score(trained_character_data.stamina)
	score += calculate_stat_score(trained_character_data.power)
	score += calculate_stat_score(trained_character_data.guts)
	score += calculate_stat_score(trained_character_data.wiz)
	# print("Stat score: ", score)
	var skill_scores: Dictionary[Variant, int] = calculate_skill_scores(trained_character_data)
	# print("Skill scores:")
	for skill: Variant in skill_scores:
		# print("%s: %d" % [skill.name, skill_scores[skill]])
		score += skill_scores[skill]
	if trained_character_data.rank_score != 0:
		if trained_character_data.rank_score == score:
			print("Score calculated correctly")
		else:
			printerr("SCORE CALCULATED WRONGLY")
	return score


func _init() -> void:
	score_table = gen_score_table()
