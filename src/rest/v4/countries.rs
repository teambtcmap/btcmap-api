use crate::db::main::area::blocking_queries::CommunityStats;
use crate::db::main::MainPool;
use crate::rest::error::RestApiError;
use crate::rest::error::RestResult as Res;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Country {
    pub id: i64,
    pub alias: String,
    pub name: String,
    pub icon: Option<String>,
    pub places_total: i64,
    pub places_verified_1y: i64,
    pub grade: i32,
}

fn calculate_grade(places_total: i64, places_verified_1y: i64) -> i32 {
    if places_total == 0 {
        return 1;
    }
    let percentage = (places_verified_1y as f64 / places_total as f64) * 100.0;
    if percentage >= 95.0 {
        5
    } else if percentage >= 75.0 {
        4
    } else if percentage >= 50.0 {
        3
    } else if percentage >= 25.0 {
        2
    } else {
        1
    }
}

#[get("/top")]
pub async fn get_top(pool: Data<MainPool>) -> Res<Vec<Country>> {
    let countries = crate::db::main::area::queries::select_top_areas_by_type(&pool, "country")
        .await
        .map_err(|_| RestApiError::database())?;

    let result: Vec<Country> = countries
        .into_iter()
        .map(|c: CommunityStats| Country {
            id: c.id,
            alias: c.alias,
            name: c.name,
            icon: c.icon_url,
            places_total: c.places_total,
            places_verified_1y: c.places_verified_1y,
            grade: calculate_grade(c.places_total, c.places_verified_1y),
        })
        .collect();

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calculate_grade_zero_places() {
        assert_eq!(calculate_grade(0, 0), 1);
    }

    #[test]
    fn test_calculate_grade_0_to_25_percent() {
        assert_eq!(calculate_grade(100, 0), 1);
        assert_eq!(calculate_grade(100, 24), 1);
    }

    #[test]
    fn test_calculate_grade_25_to_50_percent() {
        assert_eq!(calculate_grade(100, 25), 2);
        assert_eq!(calculate_grade(100, 49), 2);
    }

    #[test]
    fn test_calculate_grade_50_to_75_percent() {
        assert_eq!(calculate_grade(100, 50), 3);
        assert_eq!(calculate_grade(100, 74), 3);
    }

    #[test]
    fn test_calculate_grade_75_to_95_percent() {
        assert_eq!(calculate_grade(100, 75), 4);
        assert_eq!(calculate_grade(100, 94), 4);
    }

    #[test]
    fn test_calculate_grade_95_plus_percent() {
        assert_eq!(calculate_grade(100, 95), 5);
        assert_eq!(calculate_grade(100, 100), 5);
    }
}
