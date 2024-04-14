use mysql_async::prelude::*;
use mysql_async::*;

pub async fn open_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT IGNORE INTO issues_master (
        issue_id, 
        project_id, 
        issue_title, 
        issue_description
    )
    SELECT 
        io.issue_id, 
        io.project_id, 
        io.issue_title, 
        io.issue_description
    FROM 
        issues_open io;
    ";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => log::error!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn assigned_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
UPDATE issues_master im
JOIN issues_assigned ia ON im.issue_id = ia.issue_id
SET im.date_issue_assigned = ia.date_assigned,
    im.issue_assignees = JSON_ARRAY(ia.issue_assignee);        
    ";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => log::error!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn closed_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    UPDATE issues_master im
    JOIN issues_closed ic ON im.issue_id = ic.issue_id
    SET
        im.issue_assignees = ic.issue_assignees,
        im.issue_linked_pr = ic.issue_linked_pr;
    ";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => log::error!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn pull_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    // let query = r#"UPDATE issues_master AS im
    // JOIN pull_requests AS pr
    // ON JSON_CONTAINS(pr.connected_issues, CONCAT('"', im.issue_id, '"'), '$')
    // SET
    //     im.issue_assignees = COALESCE(im.issue_assignees, JSON_ARRAY(pr.pull_author)),
    //     im.issue_linked_pr = COALESCE(im.issue_linked_pr, pr.pull_id)
    // WHERE
    //     (im.issue_assignees IS NULL OR JSON_LENGTH(im.issue_assignees) = 0)  
    //     OR im.issue_linked_pr IS NULL;"#;

    // match conn.query_drop(query).await {
    //     Ok(_) => (),
    //     Err(e) => log::error!("Error: {:?}", e),
    // };

    Ok(())
}

pub async fn master_project(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO projects (project_id, issues_list)
    SELECT 
        project_id,
        JSON_ARRAY(
            GROUP_CONCAT(issue_id)
        )
    FROM 
        issues_master
    GROUP BY 
        project_id
    ON DUPLICATE KEY UPDATE
        issues_list = VALUES(issues_list);
    ";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => log::error!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn master_project_incl_budget(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO projects (project_id, issues_list, participants_list)
    SELECT 
        project_id,
        JSON_ARRAYAGG(issue_id) AS issues_list,
        (
            SELECT JSON_ARRAYAGG(j.assignee)
            FROM (
                SELECT DISTINCT JSON_UNQUOTE(JSON_EXTRACT(ja.value, '$')) AS assignee
                FROM issues_master, JSON_TABLE(issue_assignees, '$[*]' COLUMNS(value JSON PATH '$')) AS ja
                WHERE issues_master.project_id = distinct_values.project_id
            ) AS j
        ) AS participants_list
    FROM (
        SELECT DISTINCT
            im.project_id,
            im.issue_id,
            im.issue_assignees
        FROM 
            issues_master im
    ) AS distinct_values
    GROUP BY 
        project_id
    ON DUPLICATE KEY UPDATE
        issues_list = VALUES(issues_list),
        participants_list = VALUES(participants_list);";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => log::error!("Error: {:?}", e),
    };

    Ok(())
}
/* pub async fn master_project(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO projects (project_id, issues_list)
    SELECT
        project_id,
        JSON_ARRAYAGG(issue_id) AS issues_list
    FROM
        issues_master
    GROUP BY
        project_id
    ON DUPLICATE KEY UPDATE
        issues_list = VALUES(issues_list);";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => log::error!("Error: {:?}", e),
    };

    Ok(())
} */
