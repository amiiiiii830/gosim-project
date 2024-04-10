CREATE TABLE projects (
    project_id VARCHAR(255) PRIMARY KEY,  -- url of a project repo
    project_logo VARCHAR(255),
    repo_stars INT,
    project_description TEXT,  -- description of the project, summary of its readme, etc.
    issues_list JSON,
    participants_list JSON,
    total_budget_allocated INT,
    total_budget_used INT,
);

CREATE TABLE issues_master (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    project_id VARCHAR(255) NOT NULL,
    issue_title VARCHAR(255) NOT NULL,
    issue_description TEXT NOT NULL,  -- description of the issue, could be truncated body text
    issue_budget INT,
    issue_assignees JSON,    
    date_issue_assigned DATETIME,   
    issue_linked_pr VARCHAR(255),    -- url of the pull_request that closed the issue, if any, or the pull_request that is linked to the issue
    issue_status TEXT,    -- default empty, or some situation odd conditions occur
    review_status ENUM('queue', 'approve', 'decline'),
    date_approved DATETIME,
    date_declined DATETIME,
    issue_budget_approved BOOLEAN
    date_budget_approved DATETIME,
);

CREATE TABLE issues_open (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    project_id VARCHAR(255) NOT NULL,
    issue_title VARCHAR(255) NOT NULL,
    issue_description TEXT NOT NULL,  -- description of the issue, could be truncated body text
    repo_stars INT NOT NULL,  
    repo_avatar VARCHAR(255)
);


CREATE TABLE issues_assigned (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_assignees JSON,    
);

CREATE TABLE issues_closed (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_assignees JSON,    
    issue_linked_pr VARCHAR(255)    -- url of the pull_request that closed the issue, if any, or the pull_request that is linked to the issue
);


CREATE TABLE pull_requests (
    pull_id VARCHAR(255) PRIMARY KEY,  -- url of pull_request
    pull_title VARCHAR(255) NOT NULL,
    pull_author VARCHAR(50) ,
    project_id VARCHAR(255) NOT NULL,
    connected_issues JSON,
    date_merged DATETIME,
);