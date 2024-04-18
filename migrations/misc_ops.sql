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