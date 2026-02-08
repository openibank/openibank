//! Arena competition repository

use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{DbResult, DbArenaCompetition, DbArenaParticipant};

pub struct ArenaRepo {
    pool: PgPool,
}

impl ArenaRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // Competition Operations
    // =========================================================================

    pub async fn create_competition(&self, comp: &DbArenaCompetition) -> DbResult<DbArenaCompetition> {
        let c = sqlx::query_as::<_, DbArenaCompetition>(
            r#"
            INSERT INTO arena_competitions (id, name, description, competition_type, status, markets,
                start_time, end_time, registration_end, initial_balance, entry_fee, prize_pool,
                max_participants, scoring_config)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#
        )
        .bind(comp.id)
        .bind(&comp.name)
        .bind(&comp.description)
        .bind(&comp.competition_type)
        .bind(&comp.status)
        .bind(&comp.markets)
        .bind(comp.start_time)
        .bind(comp.end_time)
        .bind(comp.registration_end)
        .bind(&comp.initial_balance)
        .bind(&comp.entry_fee)
        .bind(&comp.prize_pool)
        .bind(comp.max_participants)
        .bind(&comp.scoring_config)
        .fetch_one(&self.pool)
        .await?;
        Ok(c)
    }

    pub async fn find_competition(&self, id: Uuid) -> DbResult<Option<DbArenaCompetition>> {
        let comp = sqlx::query_as::<_, DbArenaCompetition>(
            "SELECT * FROM arena_competitions WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(comp)
    }

    pub async fn list_competitions(&self, status: Option<&str>, limit: i64, offset: i64) -> DbResult<Vec<DbArenaCompetition>> {
        let comps = if let Some(s) = status {
            sqlx::query_as::<_, DbArenaCompetition>(
                "SELECT * FROM arena_competitions WHERE status = $1 ORDER BY start_time DESC LIMIT $2 OFFSET $3"
            )
            .bind(s)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DbArenaCompetition>(
                "SELECT * FROM arena_competitions ORDER BY start_time DESC LIMIT $1 OFFSET $2"
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(comps)
    }

    pub async fn list_active(&self) -> DbResult<Vec<DbArenaCompetition>> {
        let comps = sqlx::query_as::<_, DbArenaCompetition>(
            "SELECT * FROM arena_competitions WHERE status IN ('registration', 'active') ORDER BY start_time"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(comps)
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> DbResult<()> {
        sqlx::query("UPDATE arena_competitions SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_prize_pool(&self, id: Uuid, prize_pool: Decimal) -> DbResult<()> {
        sqlx::query("UPDATE arena_competitions SET prize_pool = $2 WHERE id = $1")
            .bind(id)
            .bind(prize_pool)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Participant Operations
    // =========================================================================

    pub async fn join_competition(&self, participant: &DbArenaParticipant) -> DbResult<DbArenaParticipant> {
        let p = sqlx::query_as::<_, DbArenaParticipant>(
            r#"
            INSERT INTO arena_participants (competition_id, user_id, wallet_id, status, entry_balance, current_balance)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(participant.competition_id)
        .bind(participant.user_id)
        .bind(participant.wallet_id)
        .bind(&participant.status)
        .bind(&participant.entry_balance)
        .bind(&participant.current_balance)
        .fetch_one(&self.pool)
        .await?;
        Ok(p)
    }

    pub async fn find_participant(&self, competition_id: Uuid, user_id: Uuid) -> DbResult<Option<DbArenaParticipant>> {
        let p = sqlx::query_as::<_, DbArenaParticipant>(
            "SELECT * FROM arena_participants WHERE competition_id = $1 AND user_id = $2"
        )
        .bind(competition_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(p)
    }

    pub async fn list_participants(&self, competition_id: Uuid) -> DbResult<Vec<DbArenaParticipant>> {
        let participants = sqlx::query_as::<_, DbArenaParticipant>(
            "SELECT * FROM arena_participants WHERE competition_id = $1 ORDER BY pnl DESC"
        )
        .bind(competition_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(participants)
    }

    pub async fn get_leaderboard(&self, competition_id: Uuid, limit: i64) -> DbResult<Vec<DbArenaParticipant>> {
        let participants = sqlx::query_as::<_, DbArenaParticipant>(
            r#"
            SELECT * FROM arena_participants
            WHERE competition_id = $1 AND status = 'active'
            ORDER BY pnl DESC, trade_count DESC
            LIMIT $2
            "#
        )
        .bind(competition_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(participants)
    }

    pub async fn update_participant_stats(
        &self,
        id: Uuid,
        current_balance: Decimal,
        pnl: Decimal,
        pnl_percent: Decimal,
        trade_count: i32,
        win_rate: Decimal,
        sharpe_ratio: Option<Decimal>,
        max_drawdown: Decimal,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE arena_participants
            SET current_balance = $2, pnl = $3, pnl_percent = $4, trade_count = $5,
                win_rate = $6, sharpe_ratio = $7, max_drawdown = $8
            WHERE id = $1
            "#
        )
        .bind(id)
        .bind(current_balance)
        .bind(pnl)
        .bind(pnl_percent)
        .bind(trade_count)
        .bind(win_rate)
        .bind(sharpe_ratio)
        .bind(max_drawdown)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_final_rank(&self, id: Uuid, rank: i32, prize_amount: Decimal) -> DbResult<()> {
        sqlx::query(
            "UPDATE arena_participants SET final_rank = $2, prize_amount = $3 WHERE id = $1"
        )
        .bind(id)
        .bind(rank)
        .bind(prize_amount)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn disqualify(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE arena_participants SET status = 'disqualified' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count_participants(&self, competition_id: Uuid) -> DbResult<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM arena_participants WHERE competition_id = $1"
        )
        .bind(competition_id)
        .fetch_one(&self.pool)
        .await?;
        let count: i64 = row.try_get("count")?;
        Ok(count)
    }
}
