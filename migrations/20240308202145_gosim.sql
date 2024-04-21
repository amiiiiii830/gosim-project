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
    issue_or_project_summary TEXT NOT NULL,
    keyword_tags JSON,
    indexed BOOLEAN NOT NULL
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


