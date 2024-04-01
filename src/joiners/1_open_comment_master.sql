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
    issues_comments ic ON io.issue_id = ic.issue_id  -- What do you mean by this line 
ON DUPLICATE KEY UPDATE
    issue_status = VALUES(issue_status);