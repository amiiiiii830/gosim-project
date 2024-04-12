use crate::db_populate::*;
use crate::issue_tracker::*;
use dotenv::dotenv;
use mysql_async::prelude::*;
use mysql_async::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub async fn open_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO issues_master (
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
        issues_open io";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn assigned_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    UPDATE issues_master im
    JOIN issues_assigned ia ON im.issue_id = ia.issue_id
    SET
        im.issue_assignees = IFNULL(im.issue_assignees, JSON_ARRAY(ia.issue_assignee)),
        im.date_issue_assigned = ia.date_assigned
    ";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
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
        im.issue_linked_pr = ic.issue_linked_pr
    ";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn pull_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r#"UPDATE issues_master AS im
    JOIN pull_requests AS pr
    ON JSON_CONTAINS(pr.connected_issues, CONCAT('"', im.issue_id, '"'), '$')
    SET
        im.issue_assignees = COALESCE(im.issue_assignees, JSON_ARRAY(pr.pull_author)),
        im.issue_linked_pr = COALESCE(im.issue_linked_pr, pr.pull_id)
    WHERE
        (im.issue_assignees IS NULL OR JSON_LENGTH(im.issue_assignees) = 0)  
        OR im.issue_linked_pr IS NULL;"#;

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn open_master_project(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO projects (
        project_id,
        project_logo,
        repo_stars,
        issues_list,
        issues_flagged
    )
    SELECT 
        im.project_id,
        io.repo_avatar AS project_logo,
        io.repo_stars,
        JSON_ARRAYAGG(im.issue_id) AS issues_list, 
        JSON_ARRAYAGG(CASE WHEN im.issue_status IS NOT NULL THEN im.issue_id ELSE NULL END) AS issues_flagged  
    FROM 
        issues_master im
    JOIN 
        issues_open io ON im.issue_id = io.issue_id
    GROUP BY 
        im.project_id, io.repo_avatar, io.repo_stars
    ON DUPLICATE KEY UPDATE
        issues_list = JSON_MERGE_PRESERVE(issues_list, VALUES(issues_list)),
        issues_flagged = JSON_MERGE_PRESERVE(issues_flagged, VALUES(issues_flagged));";

    match conn.query_drop(query).await {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}
