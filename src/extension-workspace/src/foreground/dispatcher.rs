//TODO: Handle emotions before map load.
use crate::prelude::*;

pub(crate) fn dispatch_events(res: Response) {
    if let Some(new_character_settings) = res.character_settings.and_then(|settings| settings.list)
    {
        //debug_log!(@f "character_settings {new_character_settings:?}");
        HeroSettings::merge(new_character_settings);
    }

    if let Some(new_hero) = res.h {
        Hero::merge(new_hero);
    }
    if let Some(new_items) = res.item {
        Items::merge(new_items);
    }
    if let Some(chat) = res.chat {
        Peers::update_from_chat_message(chat);
    }
    if let Some(mut members) = res.members {
        //Remove hero from members.
        members.retain(|member| Hero::get().char_id != member.id);

        Peers::update(members);
    }
    if let Some(friends) = res.friends {
        Peers::update(friends);
    }
    if let Some(new_party) = res.party {
        Party::merge(new_party);
    }
    if let Some(new_business_cards) = res.business_cards {
        Peers::update_from_business_cards(new_business_cards);
    }
    if let Some(new_others) = res.other {
        Others::merge(new_others);
    }
    if let Some(new_world_config) = res.world_config {
        WorldConfig::merge(new_world_config);
    }
    if let Some(task) = res.t {
        on_task(task);
    }
    if let Some(new_emo) = res.emo {
        on_emo(new_emo);
    }
    // TODO: This should probably get moved before the others dispatch since we need the map name
    // if a peer is on the map we enter.
    // This will need to be fixed whenever the peer's map_name gets updated only on other.action == "CREATE"
    if let Some(new_town) = res.town {
        Town::merge(new_town);
    }
    if let Some(new_gateways) = res.gateways {
        Gateways::on_gateways(new_gateways);
    }
    if let Some(new_collisions) = res.collisions {
        MapCollisions::on_collisions(new_collisions);
    }
    if let Some(npcs_del) = res.npcs_del {
        NpcCollisions::on_npcs_del(&npcs_del);
        Npcs::on_npcs_del(&npcs_del);
    }
    if let Some(new_npc_tpls) = res.npc_tpls {
        NpcTemplates::on_npc_tpls(new_npc_tpls);
    }
    if let Some(new_npcs) = res.npcs {
        NpcCollisions::on_npcs(&new_npcs);
        Npcs::on_npcs(new_npcs);
    }
}

//TODO: Extract tasks into an enum.
fn on_task(task: String) {
    if task != wasm_bindgen::intern(s!("reload")) {
        return;
    }

    Hero::reload();
    Others::reload();
    Town::reload();
    Npcs::on_clear();
    NpcTemplates::on_clear();
    MapCollisions::reload();
    NpcCollisions::reload();
    Gateways::reload();
}

fn on_emo(emotions: Vec<Emotion>) {
    emotions
        .into_iter()
        .for_each(|emotion| Others::init_emotion(emotion));
}
