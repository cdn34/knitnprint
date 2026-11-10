use std::collections::BTreeSet;

use serde_json::Value;

pub const AREA_PRICE_MINOR: i64 = 100;

/// Text and photo on the same product side and print area share one charge.
pub fn area_count(customization: Option<&Value>) -> usize {
    let Some(customization) = customization else {
        return 0;
    };
    let version = customization
        .get("version")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    if version >= 4 {
        return customization
            .get("areas")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|area| has_text(area) || has_photo(area))
            .map(|area| {
                (
                    area.get("view_id")
                        .and_then(Value::as_str)
                        .unwrap_or("view-front"),
                    area.get("area_id")
                        .and_then(Value::as_str)
                        .unwrap_or("area-1"),
                )
            })
            .collect::<BTreeSet<_>>()
            .len();
    }

    // Earlier saved designs store their text and photo directly on the design.
    ["text", "photo"]
        .into_iter()
        .filter(|kind| match *kind {
            "text" => has_text(customization),
            _ => has_photo(customization),
        })
        .filter_map(|kind| customization.get(kind))
        .map(|element| {
            element
                .get("area_id")
                .and_then(Value::as_str)
                .unwrap_or("area-1")
        })
        .collect::<BTreeSet<_>>()
        .len()
}

pub fn unit_price_minor(base_price_minor: i64, customization: Option<&Value>) -> Option<i64> {
    let areas = i64::try_from(area_count(customization)).ok()?;
    base_price_minor.checked_add(areas.checked_mul(AREA_PRICE_MINOR)?)
}

fn has_text(area: &Value) -> bool {
    area.get("text")
        .and_then(|text| text.get("content"))
        .and_then(Value::as_str)
        .is_some_and(|content| !content.trim().is_empty())
}

fn has_photo(area: &Value) -> bool {
    area.get("photo").is_some_and(Value::is_object)
}

#[cfg(test)]
mod tests {
    use super::{area_count, unit_price_minor};
    use serde_json::json;

    #[test]
    fn empty_areas_are_free_and_text_and_photo_share_one_charge() {
        for area in [
            json!({"text":{"content":"Hello"}}),
            json!({"photo":{"media_id":"photo"}}),
            json!({"text":{"content":"Hello"},"photo":{"media_id":"photo"}}),
        ] {
            let design = json!({"version":7,"areas":[area]});
            assert_eq!(unit_price_minor(2000, Some(&design)), Some(2100));
        }
        assert_eq!(unit_price_minor(2000, None), Some(2000));
        assert_eq!(
            area_count(Some(
                &json!({"version":7,"areas":[{}, {"text":{"content":"  "}}]})
            )),
            0
        );
    }

    #[test]
    fn charges_distinct_areas_on_each_side_once() {
        let design = json!({"version":7,"areas":[
            {"view_id":"front","area_id":"chest","text":{"content":"Hello"}},
            {"view_id":"front","area_id":"chest","photo":{"media_id":"photo"}},
            {"view_id":"front","area_id":"pocket","text":{"content":"Hi"}},
            {"view_id":"back","area_id":"chest","text":{"content":"Bye"}},
            {"view_id":"back","area_id":"empty"}
        ]});
        assert_eq!(area_count(Some(&design)), 3);
        assert_eq!(unit_price_minor(2000, Some(&design)), Some(2300));
    }

    #[test]
    fn supports_earlier_saved_designs_and_checks_price_overflow() {
        let combined = json!({"photo":{},"text":{"content":"Hello"}});
        assert_eq!(area_count(Some(&combined)), 1);
        let separate = json!({"version":3,"photo":{"area_id":"one"},"text":{"area_id":"two","content":"Hello"}});
        assert_eq!(area_count(Some(&separate)), 2);
        assert_eq!(unit_price_minor(i64::MAX, Some(&combined)), None);
    }
}
