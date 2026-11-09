use super::*;

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn h169_added_actual_wall_collision_uses_mob_damage_source() {
    with_game(|game| {
        assert!(game.run_command("fill 6 298 7 6 303 10 stone"));
        assert!(game.run_command(r#"summon husk 4.5 300 8.5 {UUID:[I;169,0,0,4],PersistenceRequired:1b,FallFlying:1b,Rotation:[-90f,0f],Motion:[2d,0d,0d],equipment:{chest:{id:"minecraft:elytra",count:1}}}"#));
        let id = game
            .persist
            .uuids
            .iter()
            .find_map(|(id, u)| (*u == [169, 0, 0, 4]).then_some(*id))
            .unwrap();
        let index = game.mobs.iter().position(|m| m.id == id).unwrap();
        assert!(
            game.mobs[index].kind.armor > 0.0,
            "real Husk armor is the bypass control"
        );
        assert_eq!(game.mobs[index].health, 20.0);
        game.world.game_time += 1;
        game.update_chunk_tickets();
        game.tick_creature_entities().unwrap();
        let m = &game.mobs[index];
        assert!(m.horizontal_collision, "actual wall collision");
        assert!(m.position.x > 4.5 && m.position.x < 6.0);
        assert_eq!(m.velocity.x, 0.0, "resolved collision, no damage knockback");
        assert_eq!(m.health, 3.0, "(2 - 0) * 10 - 3, bypassing the real armor");
        let damage = m.last_damage.unwrap().0;
        assert_eq!(damage.kind, horizon_client::mobs::DamageKind::FlyIntoWall);
        assert!(damage.bypasses_armor() && damage.no_knockback());
        assert!(!damage.is_fire() && !damage.is_projectile());
        assert_eq!(source_damage_type(damage), "minecraft:fly_into_wall");
        assert_eq!(game.uuid_for(id), [169, 0, 0, 4]);
    });
}
