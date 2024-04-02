-- //consolidate issues_open and issues_closed, pull_request, participants into projects
-- projects.participants_list are the login_id of participants who have participated in the project, they're in issues_closed.issue_assignees,  projects.issues_flagged are the array of issues that have been flagged in issues_master, i.e. issues with non-null issue_status
-- projects.project_logo is the avatar of the project, it's in issues_open.repo_avatar
-- Create or update the projects table with data from the issues master table and the issue's open table 
-- You don't handle. Project description. Participants list. Total budget allocated. And total budget used fueled at this time. 

CREATE TABLE projects (
    project_id VARCHAR(255) PRIMARY KEY,  -- url of a project repo
    project_logo VARCHAR(255),
    repo_stars INT,
    project_description TEXT,  -- description of the project, summary of its readme, etc.
    issues_list JSON,
    issues_flagged JSON,
    participants_list JSON,
    total_budget_allocated INT,
    total_budget_used INT
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
    issues_flagged = JSON_MERGE_PRESERVE(issues_flagged, VALUES(issues_flagged));