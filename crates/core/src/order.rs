use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderKey {
    pub display_id: String,
    pub urgent: bool,
    pub position: i64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OrderError {
    #[error("card not in column: {display_id}")]
    NotInColumn { display_id: String },
    #[error("cards in different columns: {left} and {right}")]
    DifferentColumn { left: String, right: String },
}

pub fn sort_column(keys: &mut [OrderKey]) {
    keys.sort_by(|a, b| {
        b.urgent
            .cmp(&a.urgent)
            .then_with(|| a.position.cmp(&b.position))
    });
}

pub fn rewrite_positions(keys: &mut [OrderKey]) {
    for (index, key) in keys.iter_mut().enumerate() {
        key.position = index as i64;
    }
}

pub fn place_urgent(keys: &mut Vec<OrderKey>, display_id: &str, urgent: bool) {
    let index = keys
        .iter()
        .position(|k| k.display_id == display_id)
        .expect("display_id must exist in column");
    keys[index].urgent = urgent;
    let key = keys.remove(index);

    let insert_at = if urgent {
        keys.iter()
            .rposition(|k| k.urgent)
            .map(|i| i + 1)
            .unwrap_or(0)
    } else {
        keys.iter().position(|k| !k.urgent).unwrap_or(keys.len())
    };

    keys.insert(insert_at, key);
    rewrite_positions(keys);
}

pub fn place_before(
    keys: &mut Vec<OrderKey>,
    display_id: &str,
    before: Option<&str>,
) -> Result<(), OrderError> {
    let index = keys
        .iter()
        .position(|k| k.display_id == display_id)
        .ok_or_else(|| OrderError::NotInColumn {
            display_id: display_id.to_string(),
        })?;

    let insert_at = match before {
        None => keys.len(),
        Some(before_id) => {
            if before_id == display_id {
                return Ok(());
            }
            keys.iter()
                .position(|k| k.display_id == before_id)
                .ok_or_else(|| OrderError::NotInColumn {
                    display_id: before_id.to_string(),
                })?
        }
    };

    let key = keys.remove(index);
    let insert_at = if index < insert_at {
        insert_at - 1
    } else {
        insert_at
    };
    keys.insert(insert_at, key);
    rewrite_positions(keys);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgent_cards_sort_above_non_urgent() {
        let mut keys = vec![
            OrderKey {
                display_id: "TASK-1".into(),
                urgent: false,
                position: 0,
            },
            OrderKey {
                display_id: "TASK-2".into(),
                urgent: true,
                position: 1,
            },
            OrderKey {
                display_id: "TASK-3".into(),
                urgent: false,
                position: 2,
            },
        ];
        sort_column(&mut keys);
        let ids: Vec<_> = keys.iter().map(|k| k.display_id.as_str()).collect();
        assert_eq!(ids, ["TASK-2", "TASK-1", "TASK-3"]);
    }

    #[test]
    fn toggling_urgent_on_moves_to_end_of_urgent_group() {
        let mut keys = vec![
            OrderKey {
                display_id: "TASK-1".into(),
                urgent: true,
                position: 0,
            },
            OrderKey {
                display_id: "TASK-2".into(),
                urgent: false,
                position: 1,
            },
        ];
        place_urgent(&mut keys, "TASK-2", true);
        assert!(keys[0].urgent && keys[1].urgent);
        assert_eq!(keys[1].display_id, "TASK-2");
        assert_eq!(keys[0].position, 0);
        assert_eq!(keys[1].position, 1);
    }

    #[test]
    fn toggling_urgent_off_moves_to_start_of_non_urgent_group() {
        let mut keys = vec![
            OrderKey {
                display_id: "TASK-1".into(),
                urgent: true,
                position: 0,
            },
            OrderKey {
                display_id: "TASK-2".into(),
                urgent: true,
                position: 1,
            },
            OrderKey {
                display_id: "TASK-3".into(),
                urgent: false,
                position: 2,
            },
            OrderKey {
                display_id: "TASK-4".into(),
                urgent: false,
                position: 3,
            },
        ];
        place_urgent(&mut keys, "TASK-1", false);
        assert_eq!(keys[0].display_id, "TASK-2");
        assert!(keys[0].urgent);
        assert_eq!(keys[1].display_id, "TASK-1");
        assert!(!keys[1].urgent);
        assert_eq!(keys[2].display_id, "TASK-3");
        assert!(!keys[2].urgent);
        assert_eq!(keys[0].position, 0);
        assert_eq!(keys[1].position, 1);
        assert_eq!(keys[2].position, 2);
        assert_eq!(keys[3].position, 3);
    }

    #[test]
    fn place_before_self_before_is_no_op() {
        let mut keys = vec![
            OrderKey {
                display_id: "TASK-1".into(),
                urgent: false,
                position: 0,
            },
            OrderKey {
                display_id: "TASK-2".into(),
                urgent: false,
                position: 1,
            },
        ];
        let before = snapshot_keys(&keys);
        place_before(&mut keys, "TASK-1", Some("TASK-1")).unwrap();
        assert_eq!(snapshot_keys(&keys), before);
    }

    #[test]
    fn place_before_moves_card_before_target() {
        let mut keys = vec![
            OrderKey {
                display_id: "TASK-1".into(),
                urgent: false,
                position: 0,
            },
            OrderKey {
                display_id: "TASK-2".into(),
                urgent: false,
                position: 1,
            },
            OrderKey {
                display_id: "TASK-3".into(),
                urgent: false,
                position: 2,
            },
        ];
        place_before(&mut keys, "TASK-3", Some("TASK-1")).unwrap();
        let ids: Vec<_> = keys.iter().map(|k| k.display_id.as_str()).collect();
        assert_eq!(ids, ["TASK-3", "TASK-1", "TASK-2"]);
        assert_eq!(keys[0].position, 0);
        assert_eq!(keys[1].position, 1);
        assert_eq!(keys[2].position, 2);
    }

    #[test]
    fn place_before_none_moves_card_to_end() {
        let mut keys = vec![
            OrderKey {
                display_id: "TASK-1".into(),
                urgent: false,
                position: 0,
            },
            OrderKey {
                display_id: "TASK-2".into(),
                urgent: false,
                position: 1,
            },
            OrderKey {
                display_id: "TASK-3".into(),
                urgent: false,
                position: 2,
            },
        ];
        place_before(&mut keys, "TASK-1", None).unwrap();
        let ids: Vec<_> = keys.iter().map(|k| k.display_id.as_str()).collect();
        assert_eq!(ids, ["TASK-2", "TASK-3", "TASK-1"]);
        assert_eq!(keys[2].position, 2);
    }

    #[test]
    fn place_before_returns_not_in_column_for_missing_display_id() {
        let mut keys = vec![OrderKey {
            display_id: "TASK-1".into(),
            urgent: false,
            position: 0,
        }];
        let err = place_before(&mut keys, "TASK-99", Some("TASK-1")).unwrap_err();
        assert_eq!(
            err,
            OrderError::NotInColumn {
                display_id: "TASK-99".into()
            }
        );
    }

    #[test]
    fn place_before_returns_not_in_column_for_missing_before_id() {
        let mut keys = vec![OrderKey {
            display_id: "TASK-1".into(),
            urgent: false,
            position: 0,
        }];
        let err = place_before(&mut keys, "TASK-1", Some("TASK-99")).unwrap_err();
        assert_eq!(
            err,
            OrderError::NotInColumn {
                display_id: "TASK-99".into()
            }
        );
    }

    fn snapshot_keys(keys: &[OrderKey]) -> Vec<(String, i64)> {
        keys.iter()
            .map(|k| (k.display_id.clone(), k.position))
            .collect()
    }

    #[test]
    fn place_before_leaves_slice_unchanged_on_missing_before_id() {
        let mut keys = vec![
            OrderKey {
                display_id: "TASK-1".into(),
                urgent: false,
                position: 0,
            },
            OrderKey {
                display_id: "TASK-2".into(),
                urgent: false,
                position: 1,
            },
        ];
        let before = snapshot_keys(&keys);
        let err = place_before(&mut keys, "TASK-1", Some("TASK-99")).unwrap_err();
        assert_eq!(
            err,
            OrderError::NotInColumn {
                display_id: "TASK-99".into()
            }
        );
        assert_eq!(snapshot_keys(&keys), before);
    }

    fn make_eight_keys(urgent_flags: [bool; 8]) -> Vec<OrderKey> {
        urgent_flags
            .into_iter()
            .enumerate()
            .map(|(i, urgent)| OrderKey {
                display_id: format!("TASK-{}", i + 1),
                urgent,
                position: i as i64,
            })
            .collect()
    }

    fn assert_sort_and_rewrite_invariants(keys: &mut [OrderKey]) {
        sort_column(keys);
        rewrite_positions(keys);

        let positions: Vec<i64> = keys.iter().map(|k| k.position).collect();
        assert_eq!(positions, (0..keys.len() as i64).collect::<Vec<_>>());

        let urgent_boundary = keys.iter().position(|k| !k.urgent);
        if let Some(boundary) = urgent_boundary {
            assert!(keys[..boundary].iter().all(|k| k.urgent));
            assert!(keys[boundary..].iter().all(|k| !k.urgent));
        } else {
            assert!(keys.iter().all(|k| k.urgent));
        }
    }

    #[test]
    fn sort_and_rewrite_positions_property_on_permutations() {
        let base = make_eight_keys([true, false, true, false, true, false, false, true]);

        let permutations: Vec<Vec<usize>> = vec![
            vec![0, 1, 2, 3, 4, 5, 6, 7],
            vec![7, 6, 5, 4, 3, 2, 1, 0],
            vec![3, 1, 7, 0, 5, 2, 6, 4],
            vec![2, 4, 6, 0, 1, 3, 5, 7],
            vec![1, 3, 5, 7, 0, 2, 4, 6],
        ];

        for perm in permutations {
            let mut keys: Vec<OrderKey> = perm
                .iter()
                .map(|&old_idx| {
                    let source = &base[old_idx];
                    OrderKey {
                        display_id: source.display_id.clone(),
                        urgent: source.urgent,
                        position: source.position,
                    }
                })
                .collect();
            assert_sort_and_rewrite_invariants(&mut keys);
        }
    }
}
