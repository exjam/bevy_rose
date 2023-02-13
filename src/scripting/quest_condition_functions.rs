use rose_data::QuestTrigger;
use rose_file_readers::{
    QsdAbilityType, QsdClanPosition, QsdCondition, QsdConditionOperator, QsdEquipmentIndex,
    QsdItem, QsdVariableType,
};

use crate::{
    bundles::ability_values_get_value,
    scripting::{
        quest::get_quest_variable, QuestFunctionContext, ScriptFunctionContext,
        ScriptFunctionResources,
    },
};

fn quest_condition_operator<T: PartialEq + PartialOrd>(
    operator: QsdConditionOperator,
    value_lhs: T,
    value_rhs: T,
) -> bool {
    match operator {
        QsdConditionOperator::Equals => value_lhs == value_rhs,
        QsdConditionOperator::GreaterThan => value_lhs > value_rhs,
        QsdConditionOperator::GreaterThanEqual => value_lhs >= value_rhs,
        QsdConditionOperator::LessThan => value_lhs < value_rhs,
        QsdConditionOperator::LessThanEqual => value_lhs <= value_rhs,
        QsdConditionOperator::NotEqual => value_lhs != value_rhs,
    }
}

fn quest_condition_ability_value(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    ability_type: QsdAbilityType,
    operator: QsdConditionOperator,
    compare_value: i32,
) -> bool {
    let character = script_context.query_player.single();

    let ability_type = script_resources
        .game_data
        .data_decoder
        .decode_ability_type(ability_type.get());
    if ability_type.is_none() {
        return false;
    }

    let current_value = ability_values_get_value(
        ability_type.unwrap(),
        character.ability_values,
        Some(character.character_info),
        Some(character.experience_points),
        Some(character.health_points),
        Some(character.inventory),
        Some(character.level),
        Some(character.mana_points),
        Some(character.move_speed),
        Some(character.skill_points),
        Some(character.stamina),
        Some(character.stat_points),
        Some(character.team),
        Some(character.union_membership),
    )
    .unwrap_or(0);

    quest_condition_operator(operator, current_value, compare_value)
}

fn quest_condition_check_switch(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    switch_id: usize,
    value: bool,
) -> bool {
    let quest_state = script_context.query_quest.single();

    if let Some(switch_value) = quest_state.quest_switches.get(switch_id) {
        return *switch_value == value;
    }

    false
}

fn quest_condition_quest_item(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    item: Option<QsdItem>,
    equipment_index: Option<QsdEquipmentIndex>,
    required_count: u32,
    operator: QsdConditionOperator,
) -> bool {
    let item_reference = item.and_then(|item| {
        script_resources
            .game_data
            .data_decoder
            .decode_item_reference(item.item_number, item.item_type)
    });

    let equipment_index = equipment_index.and_then(|equipment_index| {
        script_resources
            .game_data
            .data_decoder
            .decode_equipment_index(equipment_index.get())
    });

    let quest_state = script_context.query_quest.single();
    let character = script_context.query_player.single();

    if let Some(equipment_index) = equipment_index {
        item_reference
            == character
                .equipment
                .get_equipment_item(equipment_index)
                .map(|item| item.item)
    } else {
        let quantity = if let Some(item_reference) = item_reference {
            if item_reference.item_type.is_quest_item() {
                // Check selected quest item
                if let Some(selected_quest_index) = quest_context.selected_quest_index {
                    quest_state
                        .get_quest(selected_quest_index)
                        .and_then(|active_quest| active_quest.find_item(item_reference))
                        .map(|quest_item| quest_item.get_quantity())
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                // Check inventory
                character
                    .inventory
                    .find_item(item_reference)
                    .and_then(|slot| character.inventory.get_item(slot))
                    .map(|inventory_item| inventory_item.get_quantity())
                    .unwrap_or(0)
            }
        } else {
            0
        };

        quest_condition_operator(operator, quantity, required_count)
    }
}

fn quest_condition_quest_variable(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    variable_type: QsdVariableType,
    variable_id: usize,
    operator: QsdConditionOperator,
    value: i32,
) -> bool {
    if let Some(variable_value) = get_quest_variable(
        script_resources,
        script_context,
        quest_context,
        variable_type,
        variable_id,
    ) {
        quest_condition_operator(operator, variable_value, value)
    } else {
        false
    }
}

fn quest_condition_select_quest(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_id: usize,
) -> bool {
    let quest_state = script_context.query_quest.single();

    if let Some(quest_index) = quest_state.find_active_quest_index(quest_id) {
        quest_context.selected_quest_index = Some(quest_index);
        return true;
    }

    false
}

fn quest_condition_clan_position(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    operator: QsdConditionOperator,
    compare_value: QsdClanPosition,
) -> bool {
    let character = script_context.query_player.single();
    let value = character
        .clan_membership
        .and_then(|clan_membership| {
            script_resources
                .game_data
                .data_decoder
                .encode_clan_member_position(clan_membership.position)
        })
        .unwrap_or(0);
    quest_condition_operator(operator, value, compare_value)
}

fn quest_condition_in_clan(
    _script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    _quest_context: &mut QuestFunctionContext,
    in_clan: bool,
) -> bool {
    let character = script_context.query_player.single();
    character.clan_membership.is_some() == in_clan
}

pub fn quest_trigger_check_conditions(
    script_resources: &ScriptFunctionResources,
    script_context: &mut ScriptFunctionContext,
    quest_context: &mut QuestFunctionContext,
    quest_trigger: &QuestTrigger,
) -> bool {
    for condition in quest_trigger.conditions.iter() {
        let result = match *condition {
            QsdCondition::AbilityValue {
                ability_type,
                operator,
                value,
            } => quest_condition_ability_value(
                script_resources,
                script_context,
                quest_context,
                ability_type,
                operator,
                value,
            ),
            QsdCondition::QuestItem {
                item,
                equipment_index,
                required_count,
                operator,
            } => quest_condition_quest_item(
                script_resources,
                script_context,
                quest_context,
                item,
                equipment_index,
                required_count,
                operator,
            ),
            QsdCondition::QuestVariable {
                variable_type,
                variable_id,
                operator,
                value,
            } => quest_condition_quest_variable(
                script_resources,
                script_context,
                quest_context,
                variable_type,
                variable_id,
                operator,
                value,
            ),
            QsdCondition::QuestSwitch { id, value } => quest_condition_check_switch(
                script_resources,
                script_context,
                quest_context,
                id,
                value,
            ),
            QsdCondition::SelectQuest { id } => {
                quest_condition_select_quest(script_resources, script_context, quest_context, id)
            }
            QsdCondition::ClanPosition { operator, value } => quest_condition_clan_position(
                script_resources,
                script_context,
                quest_context,
                operator,
                value,
            ),
            QsdCondition::HasClan { has_clan } => {
                quest_condition_in_clan(script_resources, script_context, quest_context, has_clan)
            }
            // Server side only conditions:
            QsdCondition::RandomPercent { .. }
            | QsdCondition::ObjectVariable { .. }
            | QsdCondition::SelectEventObject { .. }
            | QsdCondition::SelectNpc { .. } => true,
            _ => {
                log::warn!("Unimplemented quest condition: {:?}", condition);
                false
            }
        };

        if !result {
            log::debug!(target: "quest", "Condition Failed: {:?}", condition);
            return false;
        } else {
            log::debug!(target: "quest", "Condition Success: {:?}", condition);
        }
    }

    true
}
