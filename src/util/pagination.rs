use crate::db::DbParam;

pub struct PaginationQuery {
    pub sql: String,
    pub params: Vec<DbParam>,
    pub top: i64,
    pub skip: i64,
}

pub fn build_paginated_json_query(
    table: &str,
    select_columns: &str,
    json_object: &str,
    where_clause: Option<&str>,
    mut where_params: Vec<DbParam>,
    top: Option<i64>,
    skip: Option<i64>,
) -> PaginationQuery {
    let top_val = top.unwrap_or(10);
    let skip_val = skip.unwrap_or(0);

    let mut params = vec![];
    params.append(&mut where_params);
    let mut clauses = vec![];

    if skip.is_some() {
        clauses.push(format!("OFFSET ${}", params.len() + 1));
        params.push(DbParam::Int64(skip_val));
    }

    if top.is_some() {
        clauses.push(format!("LIMIT ${}", params.len() + 1));
        params.push(DbParam::Int64(top_val));
    }

    let clause_str = if !clauses.is_empty() {
        format!(" {}", clauses.join(" "))
    } else {
        String::new()
    };

    let where_str = if let Some(where_clause) = where_clause {
        format!(" WHERE {}", where_clause)
    } else {
        String::new()
    };

    let sql = format!(
        "WITH page AS (
            SELECT
                {select_columns}
            FROM \"{table}\"
            {where_str} {clause_str}
        )\n\
        SELECT (
                SELECT
                    COUNT(*)
                FROM \"{table}\"
                {where_str}
            ) AS total,\n\
            COALESCE(
                json_agg(json_build_object({json_object})),
                '[]'
            ) AS data_json\n\
        FROM page;",
        select_columns = select_columns,
        table = table,
        where_str = where_str,
        clause_str = clause_str,
        json_object = json_object
    );

    PaginationQuery {
        sql,
        params,
        top: top_val,
        skip: skip_val,
    }
}
