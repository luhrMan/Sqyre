use super::*;

#[test]
fn sort_hits_uses_row_band_then_x() {
    let mut pts = vec![
        named("b", 20, 10, 0, 0),
        named("a", 5, 12, 0, 0), // same band (abs dy <= 5), lower x → first
        named("c", 1, 30, 0, 0), // next row
    ];
    sort_hits(&mut pts, &MatchOrder::default());
    assert_eq!(
        pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn sort_hits_respects_match_order() {
    let mut pts = vec![
        named("a", 5, 10, 0, 0),
        named("b", 20, 12, 0, 0),
        named("c", 1, 30, 0, 0),
    ];
    sort_hits(
        &mut pts,
        &MatchOrder {
            grouping: MatchGrouping::Row,
            horizontal: "right_to_left".into(),
            vertical: "top_to_bottom".into(),
        },
    );
    assert_eq!(
        pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        vec!["b", "a", "c"]
    );

    let mut pts = vec![
        named("a", 10, 5, 0, 0),
        named("b", 12, 20, 0, 0),
        named("c", 30, 1, 0, 0),
    ];
    sort_hits(
        &mut pts,
        &MatchOrder {
            grouping: MatchGrouping::Column,
            horizontal: "left_to_right".into(),
            vertical: "top_to_bottom".into(),
        },
    );
    assert_eq!(
        pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn sort_hits_total_order_chain_band() {
    // Points within 5px of each other pairwise but span >5px overall — old
    // conditional band compare could violate transitivity and panic in sort.
    let mut pts = vec![
        named("a", 0, 0, 0, 0),
        named("b", 10, 4, 0, 0),
        named("c", 5, 8, 0, 0),
    ];
    sort_hits(&mut pts, &MatchOrder::default());
    assert_eq!(pts.len(), 3);
}
