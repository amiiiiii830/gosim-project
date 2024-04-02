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
    merged_by VARCHAR(50) ,
    pull_status TEXT    -- default empty, or some situation exposed by conflicting information
);


INSERT INTO issues_master (
    issue_id,
    issue_assignees,
    issue_linked_pr,
    project_id,
    issue_title,
    issue_description
)
SELECT 
    ic.issue_id, 
    ic.issue_assignees,
    ic.issue_linked_pr,
    im.project_id,  -- Get the project_id from issues_master
    im.issue_title,  -- Get the project_id from issues_master
    im.issue_description  -- Get the project_id from issues_master
FROM 
    issues_closed ic
JOIN 
    issues_master im ON ic.issue_id = im.issue_id
ON DUPLICATE KEY UPDATE
    issue_assignees = VALUES(issue_assignees),
    issue_linked_pr = VALUES(issue_linked_pr);    
    

    UPDATE issues_master AS im
JOIN pull_requests AS pr
ON JSON_CONTAINS(pr.connected_issues, CONCAT('"', im.issue_id, '"'), '$')
SET
    im.issue_assignees = COALESCE(im.issue_assignees, JSON_ARRAY(pr.pull_author)),
    im.issue_linked_pr = COALESCE(im.issue_linked_pr, pr.pull_id)
WHERE
    (im.issue_assignees IS NULL OR JSON_LENGTH(im.issue_assignees) = 0)  
    OR im.issue_linked_pr IS NULL; 

-- add pr.pull_author to issue_assignees only if it's not already there, you can use JSON_ARRAY_APPEND and JSON_CONTAINS:

    UPDATE issues_master AS im
    JOIN pull_requests AS pr
    ON JSON_CONTAINS(pr.connected_issues, CONCAT('"', im.issue_id, '"'), '$')
    SET
        im.issue_assignees = CASE 
            WHEN im.issue_assignees IS NULL THEN JSON_ARRAY(pr.pull_author)
            WHEN JSON_CONTAINS(im.issue_assignees, CONCAT('"', pr.pull_author, '"'), '$') THEN im.issue_assignees
            ELSE JSON_ARRAY_APPEND(im.issue_assignees, '$', pr.pull_author)
        END,
        im.issue_linked_pr = COALESCE(im.issue_linked_pr, pr.pull_id)
    WHERE
        (im.issue_assignees IS NULL OR JSON_LENGTH(im.issue_assignees) = 0)  
        OR im.issue_linked_pr IS NULL; 

