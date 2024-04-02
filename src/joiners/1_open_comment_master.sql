CREATE TABLE issues_master (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    project_id VARCHAR(255) NOT NULL,
    issue_title VARCHAR(255) NOT NULL,
    issue_description TEXT NOT NULL,  -- description of the issue, could be truncated body text
    issue_budget INT,
    issue_assignees JSON,    
    issue_linked_pr VARCHAR(255),    -- url of the pull_request that closed the issue, if any, or the pull_request that is linked to the issue
    issue_status TEXT,    -- default empty, or some situation identified by AI summarizing the issue's comments
    review_status ENUM('queue', 'approve', 'decline'),
    issue_budget_approved BOOLEAN
);

CREATE TABLE issues_open (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    project_id VARCHAR(255) NOT NULL,
    issue_title VARCHAR(255) NOT NULL,
    issue_description TEXT NOT NULL,  -- description of the issue, could be truncated body text
    repo_stars INT NOT NULL,  
    repo_avatar VARCHAR(255)
);

CREATE TABLE issues_comments (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_status TEXT    -- default empty, or some situation identified by AI summarizing the issue's comments
);


INSERT INTO issues_master (
    issue_id, 
    project_id, 
    issue_title, 
    issue_description, 
    issue_status
)
SELECT 
    io.issue_id, 
    io.project_id, 
    io.issue_title, 
    io.issue_description, 
    ic.issue_status
FROM 
    issues_open io
JOIN 
    issues_comments ic ON io.issue_id = ic.issue_id 
ON DUPLICATE KEY UPDATE
    issue_status = VALUES(issue_status);