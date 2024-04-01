CREATE TABLE participants (
    login_id VARCHAR(255) PRIMARY KEY,  -- url of a project repo
    email  VARCHAR(255),
    in_event_status ENUM('zero', 'single', 'multiple', 'banned'),
    his_issues_list JSON
);

CREATE TABLE projects (
    project_id VARCHAR(255) PRIMARY KEY,  -- url of a project repo
    project_logo VARCHAR(255),
    repo_stars INT,
    project_description TEXT,  -- description of the project, summary of its readme, etc.
    issues_list JSON,
    issues_flagged JSON,
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


CREATE TABLE issues_closed (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_assignees JSON,    
    issue_linked_pr VARCHAR(255)    -- url of the pull_request that closed the issue, if any, or the pull_request that is linked to the issue
);

CREATE TABLE issues_comments (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_status TEXT    -- default empty, or some situation identified by AI summarizing the issue's comments
);


CREATE TABLE pull_requests (
    pull_id VARCHAR(255) PRIMARY KEY,  -- url of pull_request
    pull_title VARCHAR(255) NOT NULL,
    pull_author VARCHAR(50) ,
    project_id VARCHAR(255) NOT NULL,
    connected_issues JSON,
    merged_by VARCHAR(50) ,
    pull_status TEXT    -- default empty, or some situation exposed by conflicting information
);