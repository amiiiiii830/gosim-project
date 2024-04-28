CREATE TABLE projects (
    project_id VARCHAR(255) PRIMARY KEY,  -- url of a project repo
    project_logo VARCHAR(255) NOT NULL,
    main_language VARCHAR(50),
    repo_stars INT NOT NULL DEFAULT 0,
    project_description TEXT,  -- description of the project, summary of its readme, etc.
    issues_list JSON,
    total_budget_allocated INT NOT NULL DEFAULT 0
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_master (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    node_id VARCHAR(20) NOT NULL,   
    project_id VARCHAR(255) NOT NULL,   
    project_logo VARCHAR(255) NOT NULL,
    main_language VARCHAR(50),
    repo_stars INT NOT NULL DEFAULT 0,
    issue_title VARCHAR(255) NOT NULL,
    issue_creator VARCHAR(50) NOT NULL,
    issue_description TEXT NOT NULL,  -- description of the issue, could be truncated body text
    issue_budget INT NOT NULL DEFAULT 0,
    issue_assignees JSON,    
    date_issue_assigned DATETIME,   
    issue_linked_pr VARCHAR(255), 
    issue_status TEXT,    -- default empty, or some situation odd conditions occur
    review_status ENUM('queue', 'approve', 'decline')  NOT NULL DEFAULT 'queue',
    date_approved DATETIME,
    date_declined DATETIME,
    issue_budget_approved BOOLEAN NOT NULL DEFAULT 0,
    date_budget_approved DATETIME
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_open (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    node_id VARCHAR(20) NOT NULL,   
    project_id VARCHAR(255) NOT NULL,
    issue_creator VARCHAR(50) NOT NULL,
    issue_title VARCHAR(255) NOT NULL,
    issue_budget INT NOT NULL DEFAULT 0,
    issue_description TEXT NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_updated (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    node_id VARCHAR(20) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE issues_repos_summarized (
    issue_or_project_id VARCHAR(255) PRIMARY KEY, -- url of an issue
    issue_or_project_summary TEXT NOT NULL,
    keyword_tags JSON,
    keyword_tags_text TEXT GENERATED ALWAYS AS (JSON_UNQUOTE(JSON_EXTRACT(keyword_tags, '$'))) STORED,
    indexed BOOLEAN NOT NULL DEFAULT 0
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

ALTER TABLE issues_repos_summarized ADD FULLTEXT (keyword_tags_text);

CREATE TABLE issues_assign_comment (
    comment_id INT AUTO_INCREMENT PRIMARY KEY,  -- id of a comment
    issue_id VARCHAR(255) NOT NULL,  -- url of an issue
    node_id VARCHAR(20) NOT NULL,   
    issue_assignees JSON,    
    comment_creator VARCHAR(50) NOT NULL, 
    comment_date DATETIME NOT NULL,  -- date of the comment
    comment_body TEXT NOT NULL  -- content of the comment
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE issues_closed (
    issue_id VARCHAR(255) PRIMARY KEY,  -- url of an issue
    issue_assignees JSON,    
    issue_linked_pr VARCHAR(255) 
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE TABLE pull_requests (
    pull_id VARCHAR(255) PRIMARY KEY,  -- url of pull_request
    pull_title VARCHAR(255) NOT NULL,
    pull_author VARCHAR(50)  NOT NULL,
    project_id VARCHAR(255) NOT NULL,
    date_merged DATETIME
) DEFAULT CHARSET=utf8mb4 COLLATE utf8mb4_unicode_ci;


UPDATE issues_master im
JOIN projects p ON im.project_id = p.project_id
SET im.main_language = p.main_language,
    im.repo_stars = p.repo_stars;


UPDATE issues_master im
JOIN projects p ON im.project_id = p.project_id
SET im.project_logo = p.project_logo


-- Add a generated column that converts the JSON array to a comma-separated string
ALTER TABLE issues_repos_summarized
ADD COLUMN keyword_tags_text TEXT GENERATED ALWAYS AS (JSON_UNQUOTE(JSON_EXTRACT(keyword_tags, '$'))) STORED;

-- Add a full-text index to the generated column
ALTER TABLE issues_repos_summarized
ADD FULLTEXT(keyword_tags_text);


WITH FilteredProjects AS (
                SELECT 
                    project_id, 
                    project_logo, 
                    repo_stars, 
                    main_language, 
                    project_description, 
                    issues_list,   
                    total_budget_allocated
                FROM 
                    projects
                WHERE LENGTH(main_language) > 0 ORDER BY main_language ASC
            ),
            TotalCount AS (
                SELECT COUNT(*) AS total_count FROM FilteredProjects
            )
            SELECT 
                fp.project_id, 
                fp.project_logo, 
                fp.repo_stars, 
                fp.main_language, 
                fp.project_description, 
                fp.issues_list,   
                fp.total_budget_allocated,
                tc.total_count
            FROM 
                FilteredProjects fp, TotalCount tc



SELECT keyword, COUNT(*) as frequency
FROM issues_repos_summarized,
     JSON_TABLE(keyword_tags, '$[*]' COLUMNS(keyword VARCHAR(255) PATH '$')) AS keywords
GROUP BY keyword
ORDER BY frequency DESC;