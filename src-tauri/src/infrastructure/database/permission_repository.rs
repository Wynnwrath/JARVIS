use crate::domain::errors::AppError;
use crate::domain::permission::PermissionPreference;
use crate::infrastructure::database::models::schema::permission_preferences::dsl as prefs;
use crate::infrastructure::database::pool::DbPool;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

pub struct PermissionRepository {
    pool: DbPool,
}

impl PermissionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn get_preference(
        &self,
        name: &str,
        path_pattern: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        let mut conn = self.pool.get().await?;
        let result = match path_pattern {
            Some(p) => prefs::permission_preferences
                .filter(prefs::tool_name.eq(name))
                .filter(prefs::path_pattern.eq(p))
                .select(prefs::decision)
                .first::<String>(&mut conn)
                .await
                .optional()?,
            None => prefs::permission_preferences
                .filter(prefs::tool_name.eq(name))
                .filter(prefs::path_pattern.is_null())
                .select(prefs::decision)
                .first::<String>(&mut conn)
                .await
                .optional()?,
        };
        Ok(result)
    }

    pub async fn set_preference(
        &self,
        name: &str,
        path_pattern: Option<&str>,
        decision_val: &str,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.get().await?;
        match path_pattern {
            Some(p) => {
                diesel::insert_into(prefs::permission_preferences)
                    .values((
                        prefs::tool_name.eq(name),
                        prefs::path_pattern.eq(p),
                        prefs::decision.eq(decision_val),
                    ))
                    .on_conflict((prefs::tool_name, prefs::path_pattern))
                    .do_update()
                    .set(prefs::decision.eq(decision_val))
                    .execute(&mut conn)
                    .await?;
            }
            None => {
                diesel::delete(
                    prefs::permission_preferences
                        .filter(prefs::tool_name.eq(name))
                        .filter(prefs::path_pattern.is_null()),
                )
                .execute(&mut conn)
                .await?;
                diesel::insert_into(prefs::permission_preferences)
                    .values((
                        prefs::tool_name.eq(name),
                        prefs::path_pattern.eq::<Option<&str>>(None),
                        prefs::decision.eq(decision_val),
                    ))
                    .execute(&mut conn)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn list_preferences(&self) -> Result<Vec<PermissionPreference>, AppError> {
        let mut conn = self.pool.get().await?;
        let rows = prefs::permission_preferences
            .select((prefs::tool_name, prefs::path_pattern, prefs::decision))
            .load::<(String, Option<String>, String)>(&mut conn)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(t, p, d)| PermissionPreference {
                tool_name: t,
                path_pattern: p,
                decision: d,
            })
            .collect())
    }

    pub async fn delete_preference(
        &self,
        name: &str,
        path_pattern: Option<&str>,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.get().await?;
        match path_pattern {
            Some(p) => {
                diesel::delete(
                    prefs::permission_preferences
                        .filter(prefs::tool_name.eq(name))
                        .filter(prefs::path_pattern.eq(p)),
                )
                .execute(&mut conn)
                .await?;
            }
            None => {
                diesel::delete(
                    prefs::permission_preferences
                        .filter(prefs::tool_name.eq(name))
                        .filter(prefs::path_pattern.is_null()),
                )
                .execute(&mut conn)
                .await?;
            }
        }
        Ok(())
    }
}
