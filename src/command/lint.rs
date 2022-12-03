use crate::model::element;
use crate::model::Element;
use crate::Connection;
use crate::Result;
use time::macros::format_description;
use time::Date;

pub async fn run(db: Connection) -> Result<()> {
    log::info!("Started linting");

    let elements: Vec<Element> = db
        .prepare(element::SELECT_ALL)?
        .query_map([], element::SELECT_ALL_MAPPER)?
        .collect::<Result<Vec<Element>, _>>()?
        .into_iter()
        .filter(|it| it.deleted_at.len() == 0)
        .collect();

    log::info!("Found {} elements", elements.len());

    let date_format = format_description!("[year]-[month]-[day]");

    for element in elements {
        let survey_date = element.osm_json["tags"]["survey:date"]
            .as_str()
            .unwrap_or("");

        if survey_date.len() > 0 {
            let parsed_date = Date::parse(survey_date, &date_format);

            if parsed_date.is_err() {
                log::error!(
                    "{} survey:date is not formatted properly: {}",
                    element.id,
                    survey_date
                );
            }
        }

        let check_date = element.osm_json["tags"]["check_date"]
            .as_str()
            .unwrap_or("");

        if check_date.len() > 0 {
            let parsed_date = Date::parse(check_date, &date_format);

            if parsed_date.is_err() {
                log::error!(
                    "{} check_date is not formatted properly: {}",
                    element.id,
                    check_date,
                );
            }
        }

        let check_date_currency_xbt = element.osm_json["tags"]["check_date:currency:XBT"]
            .as_str()
            .unwrap_or("");

        if check_date_currency_xbt.len() > 0 {
            let parsed_date = Date::parse(check_date_currency_xbt, &date_format);

            if parsed_date.is_err() {
                log::error!(
                    "{} check_date:currency:XBT is not formatted properly: {}",
                    element.id,
                    check_date_currency_xbt,
                );
            }
        }

        let payment_lighting = element.osm_json["tags"]["payment:lighting"]
            .as_str()
            .unwrap_or("");

        if payment_lighting.len() > 0 {
            log::error!("{} Spelling issue: payment:lighting", element.id);
        }
    }

    log::info!("Finished linting");

    Ok(())
}
