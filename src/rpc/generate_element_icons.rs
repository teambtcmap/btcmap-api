use crate::{
    conf::Conf,
    db::{self, admin::queries::Admin},
    discord,
    element::Element,
    osm::overpass::OverpassElement,
    Result,
};
use deadpool_sqlite::Pool;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct Params {
    from_element_id: i64,
    to_element_id: i64,
}

#[derive(Serialize)]
pub struct Res {
    pub updated_elements: Vec<UpdatedElement>,
    pub time_s: f64,
}

#[derive(Serialize)]
pub struct UpdatedElement {
    pub id: i64,
    pub osm_url: String,
    pub old_icon: String,
    pub new_icon: String,
}

pub async fn run(params: Params, admin: &Admin, pool: &Pool, conf: &Conf) -> Result<Res> {
    let started_at = OffsetDateTime::now_utc();
    let updated_elements = pool
        .get()
        .await?
        .interact(move |conn| {
            generate_element_icons(params.from_element_id, params.to_element_id, conn)
        })
        .await??;
    let time_s = (OffsetDateTime::now_utc() - started_at).as_seconds_f64();
    discord::post_message(
        &conf.discord_webhook_api,
        format!(
            "Admin {} generated element icons (id range {}..{}, elements affected: {})",
            admin.name,
            params.from_element_id,
            params.to_element_id,
            updated_elements.len(),
        ),
    )
    .await;
    Ok(Res {
        updated_elements,
        time_s,
    })
}

fn generate_element_icons(
    from_element_id: i64,
    to_element_id: i64,
    conn: &Connection,
) -> Result<Vec<UpdatedElement>> {
    let mut updated_elements = vec![];
    for element_id in from_element_id..=to_element_id {
        let Ok(element) = db::element::queries::select_by_id(element_id, conn) else {
            continue;
        };
        let old_icon = element.tag("icon:android").as_str().unwrap_or_default();
        let new_icon = element.overpass_data.generate_android_icon();
        if old_icon != new_icon {
            Element::set_tag(element.id, "icon:android", &new_icon.clone().into(), conn)?;
            updated_elements.push(UpdatedElement {
                id: element_id,
                osm_url: element.osm_url(),
                old_icon: old_icon.into(),
                new_icon,
            });
        }
    }
    Ok(updated_elements)
}

impl OverpassElement {
    pub fn generate_android_icon(&self) -> String {
        let amenity = self.tag("amenity");
        let cuisine = self.tag("cuisine");
        let tourism = self.tag("tourism");
        let shop = self.tag("shop");
        let office = self.tag("office");
        let leisure = self.tag("leisure");
        let healthcare = self.tag("healthcare");
        let healthcare_speciality = self.tag("healthcare:speciality");
        let building = self.tag("building");
        let sport = self.tag("sport");
        let craft = self.tag("craft");
        let company = self.tag("company");
        let telecom = self.tag("telecom");
        let school = self.tag("school");
        let place = self.tag("place");
        let landuse = self.tag("landuse");
        let club = self.tag("club");
        let playground = self.tag("playground");
        let industrial = self.tag("industrial");
        let historic = self.tag("historic");
        let public_transport = self.tag("public_transport");
        let man_made = self.tag("man_made");
        let waterway = self.tag("waterway");
        let rental = self.tag("rental");
        let attraction = self.tag("attraction");
        let golf = self.tag("golf");
        let shelter_type = self.tag("shelter_type");
        let aeroway = self.tag("aeroway");
        let highway = self.tag("highway");
        let aerialway = self.tag("aerialway");
        let barrier = self.tag("barrier");
        let military = self.tag("military");

        let mut icon_id: &str = "question_mark";

        if !shop.is_empty() {
            icon_id = "storefront";
        }

        if !office.is_empty() {
            icon_id = "business"
        }

        if !healthcare.is_empty() {
            icon_id = "medical_services"
        }

        if !craft.is_empty() {
            icon_id = "construction"
        }

        if !playground.is_empty() {
            icon_id = "attractions"
        }

        if !industrial.is_empty() {
            icon_id = "factory"
        }

        if !attraction.is_empty() {
            icon_id = "attractions"
        }

        if !shelter_type.is_empty() {
            icon_id = "roofing"
        }

        if !aeroway.is_empty() {
            icon_id = "paragliding"
        }

        if landuse == "retail" {
            icon_id = "storefront"
        }

        if landuse == "residential" {
            icon_id = "home"
        }

        if landuse == "farmyard" {
            icon_id = "agriculture"
        }

        if landuse == "farmland" {
            icon_id = "agriculture"
        }

        if landuse == "farm" {
            icon_id = "agriculture"
        }

        if landuse == "industrial" {
            icon_id = "factory"
        }

        if landuse == "cemetery" {
            icon_id = "church"
        }

        if building == "commercial" {
            icon_id = "business"
        }

        if building == "office" {
            icon_id = "business"
        }

        if building == "retail" {
            icon_id = "storefront"
        }

        if building == "church" {
            icon_id = "church"
        }

        if building == "school" {
            icon_id = "school"
        }

        if building == "industrial" {
            icon_id = "factory"
        }

        if building == "stadium" {
            icon_id = "stadium"
        }

        if building == "farm" {
            icon_id = "storefront"
        }

        if building == "apartments" {
            icon_id = "hotel"
        }

        if building == "dormitory" {
            icon_id = "hotel"
        }

        if building == "warehouse" {
            icon_id = "warehouse";
        }

        if office == "yes" {
            icon_id = "business"
        }

        if office == "company" {
            icon_id = "business"
        }

        if office == "it" {
            icon_id = "computer"
        }

        if office == "lawyer" {
            icon_id = "balance"
        }

        if office == "accountant" {
            icon_id = "attach_money"
        }

        if office == "architect" {
            icon_id = "architecture"
        }

        if office == "educational_institution" {
            icon_id = "school"
        }

        if office == "advertising_agency" {
            icon_id = "business"
        }

        if office == "estate_agent" {
            icon_id = "home"
        }

        if office == "therapist" {
            icon_id = "medical_services"
        }

        if office == "coworking" {
            icon_id = "group"
        }

        if office == "physician" {
            icon_id = "medical_services"
        }

        if office == "marketing" {
            icon_id = "business"
        }

        if office == "surveyor" {
            icon_id = "business"
        }

        if office == "financial" {
            icon_id = "attach_money"
        }

        if office == "association" {
            icon_id = "group"
        }

        if office == "engineer" {
            icon_id = "engineering"
        }

        if office == "telecommunication" {
            icon_id = "cell_tower"
        }

        if office == "coworking_space" {
            icon_id = "group"
        }

        if office == "construction" {
            icon_id = "engineering"
        }

        if office == "tax_advisor" {
            icon_id = "attach_money"
        }

        if office == "construction_company" {
            icon_id = "engineering"
        }

        if office == "travel_agent" {
            icon_id = "tour"
        }

        if office == "insurance" {
            icon_id = "business"
        }

        if office == "ngo" {
            icon_id = "business"
        }

        if office == "newspaper" {
            icon_id = "newspaper"
        }

        if office == "trade" {
            icon_id = "business"
        }

        if office == "private" {
            icon_id = "business"
        }

        if office == "guide" {
            icon_id = "tour"
        }

        if office == "foundation" {
            icon_id = "business"
        }

        if office == "web_design" {
            icon_id = "design_services"
        }

        if office == "graphic_design" {
            icon_id = "design_services"
        }

        if office == "limousine_service" {
            icon_id = "local_taxi"
        }

        if office == "translator" {
            icon_id = "translate"
        }

        if office == "charity" {
            icon_id = "group"
        }

        if tourism == "hotel" {
            icon_id = "hotel";
        }

        if tourism == "attraction" {
            icon_id = "tour";
        }

        if tourism == "guest_house" {
            icon_id = "hotel";
        }

        if tourism == "apartment" {
            icon_id = "hotel";
        }

        if tourism == "hostel" {
            icon_id = "hotel";
        }

        if tourism == "chalet" {
            icon_id = "chalet";
        }

        if tourism == "camp_site" {
            icon_id = "camping";
        }

        if tourism == "camp_pitch" {
            icon_id = "camping";
        }

        if tourism == "gallery" {
            icon_id = "palette";
        }

        if tourism == "artwork" {
            icon_id = "palette";
        }

        if tourism == "information" {
            icon_id = "info_outline";
        }

        if tourism == "museum" {
            icon_id = "museum";
        }

        if tourism == "motel" {
            icon_id = "hotel";
        }

        if tourism == "spa" {
            icon_id = "spa";
        }

        if tourism == "theme_park" {
            icon_id = "attractions";
        }

        if tourism == "alpine_hut" {
            icon_id = "cottage";
        }

        if tourism == "caravan_site" {
            icon_id = "airport_shuttle";
        }

        if tourism == "zoo" {
            icon_id = "pets";
        }

        if shop == "general" {
            icon_id = "storefront";
        }

        if shop == "computer" {
            icon_id = "computer";
        }

        if shop == "clothes" {
            icon_id = "storefront";
        }

        if shop == "jewelry" {
            icon_id = "diamond";
        }

        if shop == "hairdresser" {
            icon_id = "content_cut";
        }

        if shop == "electronics" {
            icon_id = "computer";
        }

        if shop == "supermarket" {
            icon_id = "local_grocery_store";
        }

        if shop == "car_repair" {
            icon_id = "car_repair";
        }

        if shop == "beauty" {
            icon_id = "spa";
        }

        if shop == "books" {
            icon_id = "menu_book";
        }

        if shop == "furniture" {
            icon_id = "chair";
        }

        if shop == "convenience" {
            icon_id = "local_grocery_store";
        }

        if shop == "gift" {
            icon_id = "card_giftcard";
        }

        if shop == "travel_agency" {
            icon_id = "luggage";
        }

        if shop == "mobile_phone" {
            icon_id = "smartphone";
        }

        if shop == "tobacco" {
            icon_id = "smoking_rooms";
        }

        if shop == "car" {
            icon_id = "directions_car";
        }

        if shop == "bakery" {
            icon_id = "bakery_dining";
        }

        if shop == "massage" {
            icon_id = "spa";
        }

        if shop == "florist" {
            icon_id = "local_florist";
        }

        if shop == "bicycle" {
            icon_id = "pedal_bike";
        }

        if shop == "bicycle" {
            icon_id = "pedal_bike";
        }

        if shop == "e-cigarette" {
            icon_id = "vaping_rooms";
        }

        if shop == "optician" {
            icon_id = "visibility";
        }

        if shop == "photo" {
            icon_id = "photo_camera";
        }

        if shop == "deli" {
            icon_id = "tapas";
        }

        if shop == "sports" {
            icon_id = "sports";
        }

        if shop == "farm" {
            icon_id = "storefront";
        }

        if shop == "art" {
            icon_id = "palette";
        }

        if shop == "music" {
            icon_id = "music_note";
        }

        if shop == "hardware" {
            icon_id = "hardware";
        }

        if shop == "copyshop" {
            icon_id = "local_printshop";
        }

        if shop == "wine" {
            icon_id = "wine_bar";
        }

        if shop == "shoes" {
            icon_id = "storefront";
        }

        if shop == "alcohol" {
            icon_id = "liquor";
        }

        if shop == "toys" {
            icon_id = "toys";
        }

        if shop == "greengrocer" {
            icon_id = "storefront";
        }

        if shop == "car_parts" {
            icon_id = "directions_car";
        }

        if shop == "tatoo" {
            icon_id = "storefront";
        }

        if shop == "pawnbroker" {
            icon_id = "attach_money";
        }

        if shop == "garden_centre" {
            icon_id = "local_florist";
        }

        if shop == "butcher" {
            icon_id = "storefront";
        }

        if shop == "variety_store" {
            icon_id = "storefront";
        }

        if shop == "printing" {
            icon_id = "local_printshop";
        }

        if shop == "laundry" {
            icon_id = "local_laundry_service";
        }

        if shop == "kiosk" {
            icon_id = "storefront";
        }

        if shop == "pet" {
            icon_id = "pets";
        }

        if shop == "cannabis" {
            icon_id = "grass";
        }

        if shop == "boutique" {
            icon_id = "storefront";
        }

        if shop == "stationery" {
            icon_id = "edit";
        }

        if shop == "pastry" {
            icon_id = "bakery_dining";
        }

        if shop == "mall" {
            icon_id = "local_mall";
        }

        if shop == "hifi" {
            icon_id = "music_note";
        }

        if shop == "estate_agent" {
            icon_id = "home";
        }

        if shop == "cosmetics" {
            icon_id = "spa";
        }

        if shop == "coffee" {
            icon_id = "coffee";
        }

        if shop == "erotic" {
            icon_id = "adult_content";
        }

        if shop == "confectionery" {
            icon_id = "cake";
        }

        if shop == "beverages" {
            icon_id = "liquor";
        }

        if shop == "video_games" {
            icon_id = "games";
        }

        if shop == "newsagent" {
            icon_id = "newspaper";
        }

        if shop == "interior_decoration" {
            icon_id = "design_services";
        }

        if shop == "electrical" {
            icon_id = "electrical_services";
        }

        if shop == "doityourself" {
            icon_id = "hardware";
        }

        if shop == "antiques" {
            icon_id = "storefront";
        }

        if shop == "watches" {
            icon_id = "watch";
        }

        if shop == "trade" {
            icon_id = "storefront";
        }

        if shop == "tea" {
            icon_id = "emoji_food_beverage";
        }

        if shop == "scuba_diving" {
            icon_id = "scuba_diving";
        }

        if shop == "musical_instrument" {
            icon_id = "music_note";
        }

        if shop == "dairy" {
            icon_id = "storefront";
        }

        if shop == "chocolate" {
            icon_id = "storefront";
        }

        if shop == "anime" {
            icon_id = "storefront";
        }

        if shop == "tyres" {
            icon_id = "trip_origin";
        }

        if shop == "second_hand" {
            icon_id = "storefront";
        }

        if shop == "perfumery" {
            icon_id = "storefront";
        }

        if shop == "nutrition_supplements" {
            icon_id = "storefront";
        }

        if shop == "motorcycle" {
            icon_id = "two_wheeler";
        }

        if shop == "lottery" {
            icon_id = "storefront";
        }

        if shop == "locksmith" {
            icon_id = "lock";
        }

        if shop == "games" {
            icon_id = "games";
        }

        if shop == "funeral_directors" {
            icon_id = "church";
        }

        if shop == "department_store" {
            icon_id = "local_mall";
        }

        if shop == "chemist" {
            icon_id = "science";
        }

        if shop == "carpet" {
            icon_id = "storefront";
        }

        if shop == "water_sports" {
            icon_id = "pool";
        }

        if shop == "water" {
            icon_id = "sports";
        }

        if shop == "video" {
            icon_id = "videocam";
        }

        if shop == "tailor" {
            icon_id = "checkroom";
        }

        if shop == "storage_rental" {
            icon_id = "warehouse";
        }

        if shop == "storage" {
            icon_id = "warehouse";
        }

        if shop == "outdoor" {
            icon_id = "outdoor_grill";
        }

        if shop == "houseware" {
            icon_id = "chair";
        }

        if shop == "herbalist" {
            icon_id = "local_florist";
        }

        if shop == "health_food" {
            icon_id = "local_florist";
        }

        if shop == "grocery" {
            icon_id = "local_grocery_store";
        }

        if shop == "food" {
            icon_id = "local_grocery_store";
        }

        if shop == "curtain" {
            icon_id = "storefront";
        }

        if shop == "boat" {
            icon_id = "sailing";
        }

        if shop == "wholesale" {
            icon_id = "local_grocery_store";
        }

        if shop == "surf" {
            icon_id = "surfing";
        }

        if shop == "swimming_pool" {
            icon_id = "pool";
        }

        if shop == "gold_buyer" {
            icon_id = "diamond";
        }

        if amenity == "restaurant" {
            icon_id = "restaurant"
        }

        if amenity == "atm" {
            icon_id = "local_atm"
        }

        if amenity == "cafe" || (amenity.contains(";") && amenity.contains("cafe")) {
            icon_id = "local_cafe"
        }

        if amenity == "bar" {
            icon_id = "local_bar"
        }

        if amenity == "bureau_de_change" {
            icon_id = "currency_exchange"
        }

        if amenity == "place_of_worship" {
            icon_id = "church"
        }

        if amenity == "fast_food" {
            icon_id = "lunch_dining"
        }

        if amenity == "bank" {
            icon_id = "account_balance"
        }

        if amenity == "dentist" {
            icon_id = "medical_services"
        }

        if amenity == "pub" {
            icon_id = "sports_bar"
        }

        if amenity == "doctors" {
            icon_id = "medical_services"
        }

        if amenity == "pharmacy" {
            icon_id = "local_pharmacy"
        }

        if amenity == "clinic" {
            icon_id = "medical_services"
        }

        if amenity == "school" {
            icon_id = "school"
        }

        if amenity == "prep_school" {
            icon_id = "school"
        }

        if amenity == "taxi" {
            icon_id = "local_taxi"
        }

        if amenity == "studio" {
            icon_id = "mic"
        }

        if amenity == "fuel" {
            icon_id = "local_gas_station"
        }

        if amenity == "car_rental" {
            icon_id = "directions_car"
        }

        if amenity == "arts_centre" {
            icon_id = "palette"
        }

        if amenity == "police" {
            icon_id = "local_police"
        }

        if amenity == "hospital" {
            icon_id = "local_hospital"
        }

        if amenity == "brothel" {
            icon_id = "adult_content"
        }

        if amenity == "veterinary" {
            icon_id = "pets"
        }

        if amenity == "university" {
            icon_id = "school"
        }

        if amenity == "college" {
            icon_id = "school"
        }

        if amenity == "car_wash" {
            icon_id = "local_car_wash"
        }

        if amenity == "nightclub" {
            icon_id = "nightlife"
        }

        if amenity == "driving_school" {
            icon_id = "directions_car"
        }

        if amenity == "boat_rental" {
            icon_id = "directions_boat"
        }

        if amenity == "vending_machine" {
            icon_id = "storefront"
        }

        if amenity == "money_transfer" {
            icon_id = "currency_exchange"
        }

        if amenity == "marketplace" {
            icon_id = "storefront"
        }

        if amenity == "ice_cream" {
            icon_id = "icecream"
        }

        if amenity == "coworking_space" {
            icon_id = "business"
        }

        if amenity == "community_centre" {
            icon_id = "group"
        }

        if amenity == "kindergarten" {
            icon_id = "child_care"
        }

        if amenity == "internet_cafe" {
            icon_id = "public"
        }

        if amenity == "recycling" {
            icon_id = "delete"
        }

        if amenity == "waste_disposal" {
            icon_id = "delete"
        }

        if amenity == "payment_centre" {
            icon_id = "currency_exchange"
        }

        if amenity == "cinema" {
            icon_id = "local_movies"
        }

        if amenity == "childcare" {
            icon_id = "child_care"
        }

        if amenity == "bicycle_rental" {
            icon_id = "pedal_bike"
        }

        if amenity == "bicycle_repair_station" {
            icon_id = "pedal_bike"
        }

        if amenity == "townhall" {
            icon_id = "group"
        }

        if amenity == "theatre" {
            icon_id = "account_balance"
        }

        if amenity == "post_office" {
            icon_id = "local_post_office"
        }

        if amenity == "payment_terminal" {
            icon_id = "currency_exchange"
        }

        if amenity == "office" {
            icon_id = "business"
        }

        if amenity == "language_school" {
            icon_id = "school"
        }

        if amenity == "charging_station" {
            icon_id = "electrical_services"
        }

        if amenity == "stripclub" {
            icon_id = "adult_content"
        }

        if amenity == "spa" {
            icon_id = "spa"
        }

        if amenity == "training" {
            icon_id = "school"
        }

        if amenity == "flight_school" {
            icon_id = "flight_takeoff"
        }

        if amenity == "motorcycle_rental" {
            icon_id = "two_wheeler"
        }

        if amenity == "dojo" {
            icon_id = "sports_martial_arts"
        }

        if amenity == "animal_breeding" {
            icon_id = "cruelty_free"
        }

        if amenity == "animal_shelter" {
            icon_id = "pets"
        }

        if amenity == "food_court" {
            icon_id = "restaurant"
        }

        if amenity == "dive_centre" {
            icon_id = "scuba_diving"
        }

        if amenity == "post_depot" {
            icon_id = "mail"
        }

        if amenity == "animal_boarding" {
            icon_id = "pets"
        }

        if amenity == "events_venue" {
            icon_id = "celebration"
        }

        if amenity == "sport_school" {
            icon_id = "sports_score"
        }

        if amenity == "casino" {
            icon_id = "casino"
        }

        if amenity == "music_school" {
            icon_id = "music_note"
        }

        if amenity == "parking" {
            icon_id = "local_parking"
        }

        if amenity == "biergarten" {
            icon_id = "sports_bar"
        }

        if amenity == "car_pooling" {
            icon_id = "car_rental"
        }

        if amenity == "student_accommodation" {
            icon_id = "home"
        }

        if amenity == "surf_school" {
            icon_id = "surfing"
        }

        if amenity == "karaoke_box" {
            icon_id = "mic"
        }

        if amenity == "hookah_lounge" {
            icon_id = "smoking_rooms";
        }

        if amenity == "library" {
            icon_id = "menu_book";
        }

        if amenity == "social_facility" {
            icon_id = "group"
        }

        if amenity == "photo_booth" {
            icon_id = "photo_camera"
        }

        if amenity == "boat_storage" {
            icon_id = "directions_boat"
        }

        if amenity == "exhibition_centre" {
            icon_id = "museum"
        }

        if amenity == "dancing_school" {
            icon_id = "nightlife"
        }

        if amenity == "toilets" {
            icon_id = "wc"
        }

        if leisure == "sports_centre" {
            icon_id = "fitness_center"
        }

        if leisure == "stadium" {
            icon_id = "stadium"
        }

        if leisure == "hackerspace" {
            icon_id = "computer"
        }

        if leisure == "fitness_centre" {
            icon_id = "fitness_center"
        }

        if leisure == "pitch" {
            icon_id = "sports"
        }

        if leisure == "resort" {
            icon_id = "beach_access"
        }

        if leisure == "park" {
            icon_id = "park"
        }

        if leisure == "beach_resort" {
            icon_id = "beach_access"
        }

        if leisure == "marina" {
            icon_id = "directions_boat"
        }

        if leisure == "golf_course" {
            icon_id = "golf_course"
        }

        if leisure == "miniature_golf" {
            icon_id = "golf_course"
        }

        if leisure == "garden" {
            icon_id = "local_florist"
        }

        if leisure == "escape_game" {
            icon_id = "games"
        }

        if leisure == "dance" {
            icon_id = "nightlife"
        }

        if leisure == "kayak_dock" {
            icon_id = "kayaking"
        }

        if leisure == "water_park" {
            icon_id = "pool"
        }

        if leisure == "horse_riding" {
            icon_id = "bedroom_baby"
        }

        if leisure == "adventure_park" {
            icon_id = "nature_people"
        }

        if leisure == "casino" {
            icon_id = "casino"
        }

        if leisure == "amusement_arcade" {
            icon_id = "videogame_asset"
        }

        if leisure == "playground" {
            icon_id = "toys"
        }

        if leisure == "indoor_play" {
            icon_id = "toys"
        }

        if leisure == "track" {
            icon_id = "minor_crash"
        }

        if leisure == "bowling_alley" {
            icon_id = "directions_walk"
        }

        if leisure == "sports_hall" {
            icon_id = "fitness_center"
        }

        if leisure == "bird_hide" {
            icon_id = "raven"
        }

        if leisure == "sauna" {
            icon_id = "sauna"
        }

        if leisure == "dog_park" {
            icon_id = "pets"
        }

        if leisure == "fitness_station" {
            icon_id = "fitness_center"
        }

        if leisure == "ice_rink" {
            icon_id = "sports_hockey"
        }

        if leisure == "nature_reserve" {
            icon_id = "park"
        }

        if healthcare == "dentist" {
            icon_id = "medical_services"
        }

        if healthcare == "doctor" {
            icon_id = "medical_services"
        }

        if healthcare == "clinic" {
            icon_id = "medical_services"
        }

        if healthcare == "pharmacy" {
            icon_id = "local_pharmacy"
        }

        if healthcare == "optometrist" {
            icon_id = "visibility"
        }

        if healthcare == "physiotherapist" {
            icon_id = "medical_services"
        }

        if healthcare_speciality == "orthodontics" {
            icon_id = "dentistry"
        }

        if sport == "scuba_diving" {
            icon_id = "scuba_diving"
        }

        if sport == "soccer" {
            icon_id = "sports_soccer"
        }

        if sport == "kitesurfing" {
            icon_id = "kitesurfing"
        }

        if sport == "surfing" {
            icon_id = "surfing"
        }

        if sport == "parachuting" {
            icon_id = "paragliding"
        }

        if sport == "fitness" {
            icon_id = "fitness_center"
        }

        if sport == "free_flying" {
            icon_id = "paragliding"
        }

        if sport == "bowling" {
            icon_id = "directions_walk"
        }

        if sport == "billiards" {
            icon_id = "golf_course"
        }

        if sport == "equestrian" {
            icon_id = "bedroom_baby"
        }

        if sport == "dance" {
            icon_id = "nightlife"
        }

        if craft == "blacksmith" {
            icon_id = "hardware"
        }

        if craft == "photographer" {
            icon_id = "photo_camera"
        }

        if craft == "hvac" {
            icon_id = "hvac"
        }

        if craft == "signmaker" {
            icon_id = "hardware"
        }

        if craft == "brewery" {
            icon_id = "sports_bar"
        }

        if craft == "confectionery" {
            icon_id = "cake";
        }

        if craft == "tiler" {
            icon_id = "grid_view";
        }

        if craft == "painter" {
            icon_id = "imagesearch_roller";
        }

        if craft == "gardener" {
            icon_id = "grass";
        }

        if craft == "metal_construction" {
            icon_id = "construction";
        }

        if craft == "carpenter" {
            icon_id = "carpenter";
        }

        if craft == "joiner" {
            icon_id = "carpenter";
        }

        if craft == "cleaning" {
            icon_id = "cleaning_services";
        }

        if craft == "electrician" {
            icon_id = "electric_bolt";
        }

        if craft == "cabinet_maker" {
            icon_id = "chair";
        }

        if craft == "jeweller" {
            icon_id = "diamond";
        }

        if craft == "winery" {
            icon_id = "wine_bar";
        }

        if craft == "electronics_repair" {
            icon_id = "build";
        }

        if craft == "caterer" {
            icon_id = "cooking";
        }

        if craft == "agricultural_engines" {
            icon_id = "agriculture";
        }

        if craft == "roofer" {
            icon_id = "roofing";
        }

        if craft == "art" {
            icon_id = "palette";
        }

        if craft == "glaziery" {
            icon_id = "window";
        }

        if craft == "window_construction" {
            icon_id = "window";
        }

        if craft == "beekeeper" {
            icon_id = "hive"
        }

        if craft == "handicraft" {
            icon_id = "volunteer_activism"
        }

        if craft == "parquet_layer" {
            icon_id = "grid_view"
        }

        if craft == "pottery" {
            icon_id = "potted_plant"
        }

        if craft == "plumber" {
            icon_id = "plumbing"
        }

        if craft == "piano_tuner" {
            icon_id = "piano"
        }

        if craft == "watchmaker" {
            icon_id = "watch"
        }

        if craft == "goldsmith" {
            icon_id = "diamond"
        }

        if craft == "printer" {
            icon_id = "local_printshop";
        }

        if craft == "builder" {
            icon_id = "construction";
        }

        if craft == "atelier" {
            icon_id = "palette";
        }

        if craft == "shoemaker" {
            icon_id = "footprint";
        }

        if craft == "stonemason" {
            icon_id = "architecture";
        }

        if craft == "sculptor" {
            icon_id = "architecture";
        }

        if company == "transport" {
            icon_id = "directions_car"
        }

        if company == "internet_shop" {
            icon_id = "shopping_cart"
        }

        if company == "construction" {
            icon_id = "construction"
        }

        if cuisine == "burger" {
            icon_id = "lunch_dining"
        }

        if cuisine == "pizza" {
            icon_id = "local_pizza"
        }

        if telecom == "data_center" {
            icon_id = "dns"
        }

        if place == "farm" {
            icon_id = "agriculture"
        }

        if school == "music" {
            icon_id = "music_note"
        }

        if club == "yes" {
            icon_id = "groups"
        }

        if club == "tech" {
            icon_id = "lan"
        }

        if club == "social" {
            icon_id = "group"
        }

        if club == "charity" {
            icon_id = "group"
        }

        if club == "sport" {
            icon_id = "sports_score"
        }

        if playground == "structure" {
            icon_id = "attractions"
        }

        if industrial == "slaughterhouse" {
            icon_id = "surgical"
        }

        if historic == "castle" {
            icon_id = "castle"
        }

        if public_transport == "station" {
            icon_id = "commute"
        }

        if man_made == "works" {
            icon_id = "factory"
        }

        if man_made == "watermill" {
            icon_id = "factory"
        }

        if man_made == "wastewater_plant" {
            icon_id = "water_pump"
        }

        if man_made == "charge_point" {
            icon_id = "electrical_services"
        }

        if waterway == "boatyard" {
            icon_id = "directions_boat"
        }

        if rental == "event" {
            icon_id = "celebration"
        }

        if attraction == "animal" {
            icon_id = "pets"
        }

        if golf == "clubhouse" {
            icon_id = "golf_course"
        }

        if highway == "services" {
            icon_id = "directions_car"
        }

        if aerialway == "gondola" {
            icon_id = "panorama"
        }

        if barrier == "lift_gate" {
            icon_id = "gate"
        }

        if military == "range" {
            icon_id = "radar"
        }

        if amenity == "fast_food" && cuisine == "ice_cream" {
            icon_id = "icecream";
        }

        if craft == "electronics_repair" && shop == "mobile_phone" {
            icon_id = "smartphone";
        }

        if craft == "electronics_repair" && shop == "computer" {
            icon_id = "computer";
        }

        icon_id.to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        db,
        element::Element,
        osm::overpass::OverpassElement,
        test::{mock_conn, mock_osm_tags},
        Result,
    };

    #[actix_web::test]
    async fn run() -> Result<()> {
        let conn = mock_conn();
        db::element::queries::insert(
            &OverpassElement {
                tags: Some(mock_osm_tags(&["golf", "clubhouse"])),
                ..OverpassElement::mock(1)
            },
            &conn,
        )?;
        db::element::queries::insert(
            &OverpassElement {
                tags: Some(mock_osm_tags(&["building", "industrial"])),
                ..OverpassElement::mock(2)
            },
            &conn,
        )?;
        super::generate_element_icons(1, 100, &conn)?;
        let elements = Element::select_all(None, &conn)?;
        assert_eq!(
            "golf_course",
            elements[0].tag("icon:android").as_str().unwrap()
        );
        assert_eq!("factory", elements[1].tag("icon:android").as_str().unwrap());
        Ok(())
    }

    #[test]
    fn generate_android_icon() {
        let element = OverpassElement {
            tags: Some(mock_osm_tags(&["golf", "clubhouse"])),
            ..OverpassElement::mock(1)
        };
        assert_eq!("golf_course", &element.generate_android_icon());
        let element = OverpassElement {
            tags: Some(mock_osm_tags(&["building", "industrial"])),
            ..OverpassElement::mock(1)
        };
        assert_eq!("factory", &element.generate_android_icon());
    }
}
