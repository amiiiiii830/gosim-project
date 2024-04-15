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

    if let Err(e) = conn.query_drop(query).await {
        log::error!(
            "Error consolidating issues_open into issues_master: {:?}",
            e
        );
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

    if let Err(e) = conn.query_drop(query).await {
        log::error!(
            "Error consolidating issues_assigned into issues_master: {:?}",
            e
        );
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

    if let Err(e) = conn.query_drop(query).await {
        log::error!(
            "Error consolidating issues_closed into issues_master: {:?}",
            e
        );
    };

    Ok(())
}

pub async fn remove_pull_by_issued_linked_pr(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r#"
    DELETE FROM pull_requests
    WHERE pull_id IN (
        SELECT issue_linked_pr FROM issues_master WHERE issue_linked_pr IS NOT NULL
    );
            "#;

    if let Err(e) = conn.query_drop(query).await {
        log::error!(
            "Error removing pull_request from issues_master by issue_linked_pr: {:?}",
            e
        );
    };

    Ok(())
}

pub async fn delete_issues_open_assigned_closed(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let queries = vec![
        r#"
        DELETE FROM issues_open;
        "#,
        r#"
        DELETE FROM issues_assigned
        WHERE issue_id IN (
            SELECT issue_id FROM issues_master WHERE issue_id IS NOT NULL
        );
        "#,
        r#"
        DELETE FROM issues_closed
        WHERE issue_id IN (
            SELECT issue_id FROM issues_master WHERE issue_id IS NOT NULL
        );
        "#,
    ];

    let msgs = vec![
        "Error deleting from issues_open",
        "Error deleting from issues_assigned",
        "Error deleting from issues_closed",
    ];

    for (query, msg) in queries.iter().zip(msgs.iter()) {
        if let Err(e) = conn.query_drop(*query).await {
            log::error!("{:?}: {:?}", msg, e);
        };
    }

    Ok(())
}
pub async fn master_project(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO projects (project_id, issues_list)
    SELECT 
        project_id,
        JSON_ARRAYAGG(issue_id)
    FROM 
        (SELECT DISTINCT project_id, issue_id FROM issues_master) AS distinct_issues
    GROUP BY 
        project_id
    ON DUPLICATE KEY UPDATE
        issues_list = VALUES(issues_list);
        ";

    if let Err(e) = conn.query_drop(query).await {
        log::error!("Error building project from issues_master: {:?}", e);
    };

    Ok(())
}

pub async fn sum_budget_to_project(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    UPDATE projects p
    JOIN (
        SELECT project_id, SUM(issue_budget) AS total_budget
        FROM issues_master
        GROUP BY project_id
    ) AS summed_budgets ON p.project_id = summed_budgets.project_id
    SET p.total_budget_allocated = summed_budgets.total_budget;";

    if let Err(e) = conn.query_drop(query).await {
        log::error!("Error summing total_budget_allocated: {:?}", e);
    };

    let query = r"
    UPDATE projects p
    JOIN (
        SELECT project_id, SUM(issue_budget) AS total_used_budget
        FROM issues_master
        WHERE issue_budget_approved = TRUE
        GROUP BY project_id
    ) AS approved_budgets ON p.project_id = approved_budgets.project_id
    SET p.total_budget_used = approved_budgets.total_used_budget;";

    if let Err(e) = conn.query_drop(query).await {
        log::error!("Error summing total_budget_used: {:?}", e);
    };
    Ok(())
}
