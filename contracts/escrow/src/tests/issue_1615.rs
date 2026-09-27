//! Regression tests for issue #1615: `get_player_stats` must correctly reflect
//! wins, losses, and draws for **both** players after match settlement.
//!
//! The bug this exercises: `update_player_stats` uses the same `Winner` enum
//! value for both players, so `Winner::Player2` was recorded as a *loss* for
//! player2 instead of a win, and `Winner::Player1` was recorded as a *win*
//! for player1 but a *loss* for player2 instead of a loss.  These tests pin
//! the correct post-settlement stats for all three outcomes so any regression
//! in the logic is immediately caught.

use super::*;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Create a funded Active match (both players have deposited) and submit the
/// given result.  Returns the match ID.
fn settle_match(
    client: &EscrowContractClient,
    env: &Env,
    player1: &Address,
    player2: &Address,
    token: &Address,
    oracle: &Address,
    game_id: &str,
    winner: &Winner,
) -> u64 {
    let mid = client.create_match(
        player1,
        player2,
        &100,
        token,
        &String::from_str(env, game_id),
        &Platform::Lichess,
    );
    client.deposit(&mid, player1);
    client.deposit(&mid, player2);
    client.submit_result(&mid, winner, oracle);
    mid
}

// ── player1 wins ─────────────────────────────────────────────────────────────

#[test]
fn test_player_stats_player1_win_records_win_for_p1_loss_for_p2() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    settle_match(
        &client,
        &env,
        &player1,
        &player2,
        &token,
        &oracle,
        "st1615a1",
        &Winner::Player1,
    );

    let stats1 = client.get_player_stats(&player1);
    assert_eq!(stats1.total_matches, 1, "p1: total_matches should be 1 after one match");
    assert_eq!(stats1.wins, 1, "p1: should have 1 win when Winner::Player1");
    assert_eq!(stats1.losses, 0, "p1: should have 0 losses when Winner::Player1");
    assert_eq!(stats1.draws, 0, "p1: should have 0 draws when Winner::Player1");
    assert_eq!(
        stats1.total_volume_staked, 100,
        "p1: total_volume_staked should equal the stake amount"
    );

    let stats2 = client.get_player_stats(&player2);
    assert_eq!(stats2.total_matches, 1, "p2: total_matches should be 1 after one match");
    assert_eq!(stats2.wins, 0, "p2: should have 0 wins when Winner::Player1");
    assert_eq!(stats2.losses, 1, "p2: should have 1 loss when Winner::Player1");
    assert_eq!(stats2.draws, 0, "p2: should have 0 draws when Winner::Player1");
    assert_eq!(
        stats2.total_volume_staked, 100,
        "p2: total_volume_staked should equal the stake amount"
    );
}

// ── player2 wins ─────────────────────────────────────────────────────────────

#[test]
fn test_player_stats_player2_win_records_win_for_p2_loss_for_p1() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    settle_match(
        &client,
        &env,
        &player1,
        &player2,
        &token,
        &oracle,
        "st1615b1",
        &Winner::Player2,
    );

    let stats1 = client.get_player_stats(&player1);
    assert_eq!(stats1.total_matches, 1, "p1: total_matches should be 1 after one match");
    assert_eq!(stats1.wins, 0, "p1: should have 0 wins when Winner::Player2");
    assert_eq!(stats1.losses, 1, "p1: should have 1 loss when Winner::Player2");
    assert_eq!(stats1.draws, 0, "p1: should have 0 draws when Winner::Player2");
    assert_eq!(
        stats1.total_volume_staked, 100,
        "p1: total_volume_staked should equal the stake amount"
    );

    let stats2 = client.get_player_stats(&player2);
    assert_eq!(stats2.total_matches, 1, "p2: total_matches should be 1 after one match");
    assert_eq!(stats2.wins, 1, "p2: should have 1 win when Winner::Player2");
    assert_eq!(stats2.losses, 0, "p2: should have 0 losses when Winner::Player2");
    assert_eq!(stats2.draws, 0, "p2: should have 0 draws when Winner::Player2");
    assert_eq!(
        stats2.total_volume_staked, 100,
        "p2: total_volume_staked should equal the stake amount"
    );
}

// ── draw ─────────────────────────────────────────────────────────────────────

#[test]
fn test_player_stats_draw_records_draw_for_both_players() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    settle_match(
        &client,
        &env,
        &player1,
        &player2,
        &token,
        &oracle,
        "st1615c1",
        &Winner::Draw,
    );

    let stats1 = client.get_player_stats(&player1);
    assert_eq!(stats1.total_matches, 1, "p1: total_matches should be 1 after one match");
    assert_eq!(stats1.wins, 0, "p1: should have 0 wins on draw");
    assert_eq!(stats1.losses, 0, "p1: should have 0 losses on draw");
    assert_eq!(stats1.draws, 1, "p1: should have 1 draw on Winner::Draw");
    assert_eq!(
        stats1.total_volume_staked, 100,
        "p1: total_volume_staked should equal the stake amount"
    );

    let stats2 = client.get_player_stats(&player2);
    assert_eq!(stats2.total_matches, 1, "p2: total_matches should be 1 after one match");
    assert_eq!(stats2.wins, 0, "p2: should have 0 wins on draw");
    assert_eq!(stats2.losses, 0, "p2: should have 0 losses on draw");
    assert_eq!(stats2.draws, 1, "p2: should have 1 draw on Winner::Draw");
    assert_eq!(
        stats2.total_volume_staked, 100,
        "p2: total_volume_staked should equal the stake amount"
    );
}

// ── accumulation across multiple matches ─────────────────────────────────────

#[test]
fn test_player_stats_accumulate_across_multiple_matches() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    // Match 1: player1 wins
    settle_match(
        &client,
        &env,
        &player1,
        &player2,
        &token,
        &oracle,
        "st1615d1",
        &Winner::Player1,
    );

    // Match 2: player2 wins
    settle_match(
        &client,
        &env,
        &player1,
        &player2,
        &token,
        &oracle,
        "st1615d2",
        &Winner::Player2,
    );

    // Match 3: draw
    settle_match(
        &client,
        &env,
        &player1,
        &player2,
        &token,
        &oracle,
        "st1615d3",
        &Winner::Draw,
    );

    let stats1 = client.get_player_stats(&player1);
    assert_eq!(stats1.total_matches, 3, "p1: three matches played");
    assert_eq!(stats1.wins, 1, "p1: one win (match 1)");
    assert_eq!(stats1.losses, 1, "p1: one loss (match 2)");
    assert_eq!(stats1.draws, 1, "p1: one draw (match 3)");
    assert_eq!(
        stats1.total_volume_staked, 300,
        "p1: total_volume_staked = 3 × 100"
    );

    let stats2 = client.get_player_stats(&player2);
    assert_eq!(stats2.total_matches, 3, "p2: three matches played");
    assert_eq!(stats2.wins, 1, "p2: one win (match 2)");
    assert_eq!(stats2.losses, 1, "p2: one loss (match 1)");
    assert_eq!(stats2.draws, 1, "p2: one draw (match 3)");
    assert_eq!(
        stats2.total_volume_staked, 300,
        "p2: total_volume_staked = 3 × 100"
    );
}

// ── default stats for a player who has never played ──────────────────────────

#[test]
fn test_player_stats_default_for_new_player() {
    let (env, contract_id, _oracle, player1, _player2, _token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let stats = client.get_player_stats(&player1);
    assert_eq!(stats.total_matches, 0);
    assert_eq!(stats.wins, 0);
    assert_eq!(stats.losses, 0);
    assert_eq!(stats.draws, 0);
    assert_eq!(stats.total_volume_staked, 0);
}
