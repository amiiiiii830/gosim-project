INSERT INTO issues_master (
    issue_id,
    project_id,
    issue_title,
    issue_creator,
    issue_description,
    issue_budget,
    issue_assignees,
    date_issue_assigned,
    issue_linked_pr,
    issue_status,
    review_status,
    date_approved,
    date_declined
)
SELECT
    DISTINCT issue_id,
    SUBSTRING_INDEX(issue_id, '/issues', 1) AS project_id,  -- Extract project URL from issue URL
    CONCAT('Issue Title ', FLOOR(RAND() * 1000) + 1) AS issue_title,
    CONCAT('User_', FLOOR(RAND() * 100) + 1) AS issue_creator,
    SUBSTRING(comment_body, 1, 255) AS issue_description,  -- Truncate the first comment to fit the issue_description
    FLOOR(RAND() * 5000) + 500 AS issue_budget,
    JSON_ARRAY(CONCAT('User_', FLOOR(RAND() * 100) + 1)) AS issue_assignees,
    NOW() AS date_issue_assigned,
    CONCAT('PR_', FLOOR(RAND() * 1000) + 1) AS issue_linked_pr,
    '' AS issue_status,
    'approve' AS review_status,  -- Static value for all entries
    NOW() AS date_approved,  -- Set approval date since review_status is 'approve'
    NULL AS date_declined,  -- No declined date since all are approved
FROM issues_comment
ON DUPLICATE KEY UPDATE
    issue_budget = VALUES(issue_budget),
    issue_assignees = VALUES(issue_assignees),
    date_issue_assigned = VALUES(date_issue_assigned),
    issue_linked_pr = VALUES(issue_linked_pr),
    issue_status = VALUES(issue_status),
    review_status = VALUES(review_status),
    date_approved = VALUES(date_approved),
    date_declined = VALUES(date_declined);


ALTER DATABASE gosim CHARACTER SET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

ALTER TABLE table_name CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

SELECT 
    table_name, 
    column_name, 
    character_set_name, 
    collation_name
FROM 
    information_schema.columns
WHERE 
    table_schema = 'gosim2';


SET SESSION group_concat_max_len = 10000; 

    INSERT INTO projects (project_id, issues_list)
    SELECT 
        project_id,
        JSON_ARRAYAGG(issue_id)
    FROM 
        issues_master
    GROUP BY 
        project_id
    ON DUPLICATE KEY UPDATE
        issues_list = merge_json_arrays(issues_list, VALUES(issues_list));


UPDATE issues_master
SET issue_linked_pr = NULL
WHERE issue_linked_pr = '';

-- randomly assign budgets to 20% of the issues
SET @total_rows = (SELECT COUNT(*) FROM issues_master);
SET @rows_to_update = ROUND(@total_rows * 0.2);
SET @sql = CONCAT('UPDATE issues_master SET issue_budget = FLOOR(50 + RAND() * 51) ORDER BY RAND() LIMIT ', @rows_to_update);

PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- randomly assign budgets to 20% of the projects
SET @total_rows = (SELECT COUNT(*) FROM projects);
SET @rows_to_update = ROUND(@total_rows * 0.2);
SET @sql = CONCAT('UPDATE projects SET total_budget_allocated = FLOOR(350 + RAND() * 51) ORDER BY RAND() LIMIT ', @rows_to_update);

PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- assign 30% of the issues to a decline
SET @total_rows = (SELECT COUNT(*) FROM issues_master);
SET @rows_to_update = ROUND(@total_rows * 0.3);
SET @sql = CONCAT('UPDATE issues_master SET review_status = \'decline\' ORDER BY RAND() LIMIT ', @rows_to_update);

PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- Calculate the total number of rows and the number of rows to update
SET @total_rows = (SELECT COUNT(*) FROM projects);
SET @rows_to_update = ROUND(@total_rows * 0.3);

-- Prepare a statement to update the projects table
SET @sql = CONCAT('UPDATE projects SET main_language = CASE 
    WHEN RAND() <= 0.1667 THEN ''Rust'' 
    WHEN RAND() > 0.1667 AND RAND() <= 0.3334 THEN ''Javascript''
    WHEN RAND() > 0.3334 AND RAND() <= 0.5001 THEN ''html''
    WHEN RAND() > 0.5001 AND RAND() <= 0.6668 THEN ''cplusplus''
    WHEN RAND() > 0.6668 AND RAND() <= 0.8335 THEN ''typescript''
    ELSE ''Go''
    END 
ORDER BY RAND() LIMIT ', @rows_to_update);

-- Execute the prepared statement
PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

-- assign 20% to be budget approved
SET @total_rows = (SELECT COUNT(*) FROM issues_master);
SET @rows_to_update = ROUND(@total_rows * 0.2);
SET @sql = CONCAT('UPDATE issues_master SET issue_budget_approved = TRUE ORDER BY RAND() LIMIT ', @rows_to_update);

PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;

select issue_id from issues_master where date_issue_assigned < '2023-10-04 13:04:00' AND issue_linked_pr IS NULL;


select issue_id from issues_master where review_status='decline' limit 5;

select issue_id, issue_budget  from issues_master where issue_budget_approved=1;

UPDATE projects p
JOIN (
    SELECT project_id, SUM(issue_budget) AS total_budget
    FROM issues_master
    GROUP BY project_id
) AS summed_budgets ON p.project_id = summed_budgets.project_id
SET p.total_budget_allocated = summed_budgets.total_budget;


UPDATE projects p
JOIN (
    SELECT project_id, SUM(issue_budget) AS total_used_budget
    FROM issues_master
    WHERE issue_budget_approved = TRUE
    GROUP BY project_id
) AS approved_budgets ON p.project_id = approved_budgets.project_id
SET p.total_budget_used = approved_budgets.total_used_budget;

SELECT pull_requests.pull_id
FROM pull_requests
JOIN issues_master
ON pull_requests.pull_id = issues_master.issue_linked_pr;


-- Step 2: Insert the records into the orphan_pull_requests table
INSERT INTO orphan_pull_requests
SELECT * FROM pull_requests
WHERE pull_id NOT IN (
    SELECT issue_linked_pr FROM issues_master WHERE issue_linked_pr IS NOT NULL
);

-- Step 3: Delete the records from the pull_requests table
DELETE FROM pull_requests
WHERE pull_id IN (
    SELECT issue_linked_pr FROM issues_master WHERE issue_linked_pr IS NOT NULL
);

UPDATE issues_master
SET issue_assignees = JSON_ARRAY()
WHERE issue_assignees IS NULL;

select issue_id, issue_description from issues_master where  project_id not in (SELECT issue_or_project_id FROM issues_repos_indexed) limit 1;

select project_id, project_description from projects where  project_id not in (SELECT issue_or_project_id FROM issues_repos_indexed) limit 1;

SELECT COUNT(DISTINCT issue_id) FROM issues_comment;


