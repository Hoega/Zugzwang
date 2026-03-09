use crate::models::chess_move::{MoveNode, MoveTreeNode};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_flat_moves(pool: &PgPool, repertoire_id: Uuid) -> Result<Vec<MoveNode>, sqlx::Error> {
    sqlx::query_as::<_, MoveNode>(
        r#"
        WITH RECURSIVE move_tree AS (
            SELECT * FROM moves WHERE repertoire_id = $1 AND parent_id IS NULL
            UNION ALL
            SELECT m.* FROM moves m
            INNER JOIN move_tree mt ON m.parent_id = mt.id
        )
        SELECT * FROM move_tree ORDER BY move_number, sort_order
        "#,
    )
    .bind(repertoire_id)
    .fetch_all(pool)
    .await
}

pub fn build_tree(flat: Vec<MoveNode>) -> Vec<MoveTreeNode> {
    use std::collections::HashMap;

    let mut children_map: HashMap<Option<Uuid>, Vec<MoveNode>> = HashMap::new();
    for node in flat {
        children_map
            .entry(node.parent_id)
            .or_default()
            .push(node);
    }

    fn build_children(
        parent_id: Option<Uuid>,
        map: &mut HashMap<Option<Uuid>, Vec<MoveNode>>,
    ) -> Vec<MoveTreeNode> {
        let nodes = match map.remove(&parent_id) {
            Some(n) => n,
            None => return vec![],
        };
        nodes
            .into_iter()
            .map(|node| {
                let id = node.id;
                let children = build_children(Some(id), map);
                MoveTreeNode { node, children }
            })
            .collect()
    }

    build_children(None, &mut children_map)
}

pub async fn get_variant_names(pool: &PgPool, repertoire_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT variant_name FROM moves WHERE repertoire_id = $1 AND variant_name IS NOT NULL ORDER BY variant_name",
    )
    .bind(repertoire_id)
    .fetch_all(pool)
    .await
}

pub async fn get_descendant_ids(pool: &PgPool, move_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        WITH RECURSIVE subtree AS (
            SELECT id FROM moves WHERE id = $1
            UNION ALL
            SELECT m.id FROM moves m INNER JOIN subtree s ON m.parent_id = s.id
        )
        SELECT id FROM subtree
        "#,
    )
    .bind(move_id)
    .fetch_all(pool)
    .await
}

pub async fn get_line_to_root(pool: &PgPool, move_id: Uuid) -> Result<Vec<MoveNode>, sqlx::Error> {
    sqlx::query_as::<_, MoveNode>(
        r#"
        WITH RECURSIVE line AS (
            SELECT * FROM moves WHERE id = $1
            UNION ALL
            SELECT m.* FROM moves m
            INNER JOIN line l ON m.id = l.parent_id
        )
        SELECT * FROM line ORDER BY move_number ASC, is_white_move DESC
        "#,
    )
    .bind(move_id)
    .fetch_all(pool)
    .await
}
