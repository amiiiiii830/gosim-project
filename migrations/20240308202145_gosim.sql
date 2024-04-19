CREATE TABLE projects (
    project_id VARCHAR(255) PRIMARY KEY,  -- url of a project repo
    project_logo VARCHAR(255),
    repo_stars INT,
    project_description TEXT,  -- description of the project, summary of its readme, etc.
    issues_list JSON,
    total_budget_allocated INT
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_master (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    project_id VARCHAR(255) NOT NULL,
    issue_title VARCHAR(255) NOT NULL,
    issue_creator VARCHAR(50) NOT NULL,
    issue_description TEXT NOT NULL,  -- description of the issue, could be truncated body text
    issue_budget INT,
    issue_assignees JSON,    
    date_issue_assigned DATETIME,   
    issue_linked_pr VARCHAR(255), 
    issue_status TEXT,    -- default empty, or some situation odd conditions occur
    review_status ENUM('queue', 'approve', 'decline'),
    date_approved DATETIME,
    date_declined DATETIME,
    issue_budget_approved BOOLEAN,
    date_budget_approved DATETIME
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_open (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    project_id VARCHAR(255) NOT NULL,
    issue_creator VARCHAR(50) NOT NULL,
    issue_title VARCHAR(255) NOT NULL,
    issue_budget INT,
    issue_description TEXT NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_repos_indexed (
    issue_or_project_id VARCHAR(255) PRIMARY KEY  -- url of an issue
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_repos_summarized (
    issue_or_project_id VARCHAR(255) PRIMARY KEY, -- url of an issue
    issue_or_project_summary TEXT NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;


CREATE TABLE issues_comment (
    comment_id INT AUTO_INCREMENT PRIMARY KEY,  -- id of a comment
    issue_id VARCHAR(255) NOT NULL,  -- url of an issue
    comment_creator VARCHAR(50) NOT NULL, 
    comment_date DATETIME NOT NULL,  -- date of the comment
    comment_body TEXT NOT NULL  -- content of the comment
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE issues_assigned (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_assignee VARCHAR(50),    
    date_assigned DATETIME   
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_closed (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_assignees JSON,    
    issue_linked_pr VARCHAR(255) 
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;


CREATE TABLE pull_requests (
    pull_id VARCHAR(255) PRIMARY KEY,  -- url of pull_request
    pull_title VARCHAR(255) NOT NULL ,
    pull_author VARCHAR(50) ,
    project_id VARCHAR(255) NOT NULL,
    date_merged DATETIME
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;


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

-- assign 30% of the issues to a decline
SET @total_rows = (SELECT COUNT(*) FROM issues_master);
SET @rows_to_update = ROUND(@total_rows * 0.3);
SET @sql = CONCAT('UPDATE issues_master SET review_status = \'decline\' ORDER BY RAND() LIMIT ', @rows_to_update);

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


