/// Email template functions
///
/// This module provides functions to generate common email templates.
use super::{send_email, EmailResult};

/// Send a password reset email
pub async fn send_password_reset_email(
    to: &str,
    username: &str,
    reset_token: &str,
    base_url: &str,
) -> EmailResult<()> {
    let reset_link = format!("{}/password-reset/{}", base_url, reset_token);

    let body_text = format!(
        r#"Hello {},

You have requested to reset your password.

Click the link below to reset your password:
{}

This link will expire in 1 hour.

If you did not request a password reset, please ignore this email.

---
Ruforo Forum
"#,
        username, reset_link
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Password Reset</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>Password Reset Request</h2>
        <p>Hello <strong>{}</strong>,</p>
        <p>You have requested to reset your password.</p>
        <p>Click the button below to reset your password:</p>
        <p style="margin: 30px 0;">
            <a href="{}"
               style="background-color: #007bff; color: white; padding: 12px 24px;
                      text-decoration: none; border-radius: 4px; display: inline-block;">
                Reset Password
            </a>
        </p>
        <p>Or copy and paste this link into your browser:</p>
        <p style="word-break: break-all; color: #007bff;">{}</p>
        <p><strong>This link will expire in 1 hour.</strong></p>
        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ddd;">
        <p style="color: #666; font-size: 0.9em;">
            If you did not request a password reset, please ignore this email.
        </p>
    </div>
</body>
</html>"#,
        username, reset_link, reset_link
    );

    send_email(to, "Password Reset Request", &body_text, Some(&body_html)).await
}

/// Send an email verification email
pub async fn send_verification_email(
    to: &str,
    username: &str,
    verification_token: &str,
    base_url: &str,
) -> EmailResult<()> {
    let verification_link = format!("{}/verify-email/{}", base_url, verification_token);

    let body_text = format!(
        r#"Hello {},

Thank you for registering!

Please verify your email address by clicking the link below:
{}

This link will expire in 24 hours.

---
Ruforo Forum
"#,
        username, verification_link
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Email Verification</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>Welcome to Ruforo Forum!</h2>
        <p>Hello <strong>{}</strong>,</p>
        <p>Thank you for registering. Please verify your email address to complete your registration.</p>
        <p style="margin: 30px 0;">
            <a href="{}"
               style="background-color: #28a745; color: white; padding: 12px 24px;
                      text-decoration: none; border-radius: 4px; display: inline-block;">
                Verify Email Address
            </a>
        </p>
        <p>Or copy and paste this link into your browser:</p>
        <p style="word-break: break-all; color: #28a745;">{}</p>
        <p><strong>This link will expire in 24 hours.</strong></p>
        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ddd;">
        <p style="color: #666; font-size: 0.9em;">
            If you did not create an account, please ignore this email.
        </p>
    </div>
</body>
</html>"#,
        username, verification_link, verification_link
    );

    send_email(
        to,
        "Verify Your Email Address",
        &body_text,
        Some(&body_html),
    )
    .await
}

/// Send a welcome email after verification
pub async fn send_welcome_email(to: &str, username: &str) -> EmailResult<()> {
    let body_text = format!(
        r#"Hello {},

Welcome to Ruforo Forum!

Your email has been verified and your account is now fully activated.

You can now log in and start participating in discussions.

---
Ruforo Forum
"#,
        username
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Welcome!</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>Welcome to Ruforo Forum!</h2>
        <p>Hello <strong>{}</strong>,</p>
        <p>Your email has been verified and your account is now fully activated.</p>
        <p>You can now log in and start participating in discussions.</p>
        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ddd;">
        <p style="color: #666; font-size: 0.9em;">
            Thank you for joining our community!
        </p>
    </div>
</body>
</html>"#,
        username
    );

    send_email(to, "Welcome to Ruforo Forum!", &body_text, Some(&body_html)).await
}

/// Send a thread reply notification email
pub async fn send_thread_reply_email(
    to: &str,
    recipient_username: &str,
    thread_title: &str,
    thread_id: i32,
    poster_username: &str,
    post_preview: &str,
    base_url: &str,
) -> EmailResult<()> {
    let thread_link = format!("{}/threads/{}", base_url, thread_id);

    // Truncate preview to 500 chars
    let preview = if post_preview.len() > 500 {
        format!("{}...", &post_preview[..500])
    } else {
        post_preview.to_string()
    };

    let body_text = format!(
        r#"Hello {},

{} has replied to a thread you're watching:

"{}"

---
{}
---

View the thread: {}

To stop receiving these emails, visit the thread and disable email notifications.

---
Ruforo Forum
"#,
        recipient_username, poster_username, thread_title, preview, thread_link
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>New Reply</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>New Reply in Watched Thread</h2>
        <p>Hello <strong>{}</strong>,</p>
        <p><strong>{}</strong> has replied to a thread you're watching:</p>
        <h3 style="color: #007bff;">{}</h3>
        <div style="background: #f8f9fa; border-left: 4px solid #007bff; padding: 15px; margin: 20px 0;">
            <p style="margin: 0; white-space: pre-wrap;">{}</p>
        </div>
        <p style="margin: 30px 0;">
            <a href="{}"
               style="background-color: #007bff; color: white; padding: 12px 24px;
                      text-decoration: none; border-radius: 4px; display: inline-block;">
                View Thread
            </a>
        </p>
        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ddd;">
        <p style="color: #666; font-size: 0.9em;">
            To stop receiving these emails, visit the thread and disable email notifications.
        </p>
    </div>
</body>
</html>"#,
        recipient_username, poster_username, thread_title, preview, thread_link
    );

    let subject = format!("Re: {}", thread_title);
    send_email(to, &subject, &body_text, Some(&body_html)).await
}

/// Send a mention notification email
pub async fn send_mention_email(
    to: &str,
    recipient_username: &str,
    mentioner_username: &str,
    thread_title: &str,
    thread_id: i32,
    post_id: i32,
    post_preview: &str,
    base_url: &str,
) -> EmailResult<()> {
    let post_link = format!("{}/threads/{}#post-{}", base_url, thread_id, post_id);

    // Truncate preview to 500 chars
    let preview = if post_preview.len() > 500 {
        format!("{}...", &post_preview[..500])
    } else {
        post_preview.to_string()
    };

    let body_text = format!(
        r#"Hello {},

{} mentioned you in a post:

"{}"

---
{}
---

View the post: {}

To stop receiving these emails, update your notification preferences in your account settings.

---
Ruforo Forum
"#,
        recipient_username, mentioner_username, thread_title, preview, post_link
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>You were mentioned</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>You were mentioned</h2>
        <p>Hello <strong>{}</strong>,</p>
        <p><strong>{}</strong> mentioned you in:</p>
        <h3 style="color: #007bff;">{}</h3>
        <div style="background: #f8f9fa; border-left: 4px solid #17a2b8; padding: 15px; margin: 20px 0;">
            <p style="margin: 0; white-space: pre-wrap;">{}</p>
        </div>
        <p style="margin: 30px 0;">
            <a href="{}"
               style="background-color: #17a2b8; color: white; padding: 12px 24px;
                      text-decoration: none; border-radius: 4px; display: inline-block;">
                View Post
            </a>
        </p>
        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ddd;">
        <p style="color: #666; font-size: 0.9em;">
            To stop receiving these emails, update your notification preferences in your account settings.
        </p>
    </div>
</body>
</html>"#,
        recipient_username, mentioner_username, thread_title, preview, post_link
    );

    let subject = format!("{} mentioned you in: {}", mentioner_username, thread_title);
    send_email(to, &subject, &body_text, Some(&body_html)).await
}

/// Send a thread author reply notification email (for thread owner, not watchers)
pub async fn send_author_reply_email(
    to: &str,
    recipient_username: &str,
    replier_username: &str,
    thread_title: &str,
    thread_id: i32,
    post_id: i32,
    post_preview: &str,
    base_url: &str,
) -> EmailResult<()> {
    let post_link = format!("{}/threads/{}#post-{}", base_url, thread_id, post_id);

    // Truncate preview to 500 chars
    let preview = if post_preview.len() > 500 {
        format!("{}...", &post_preview[..500])
    } else {
        post_preview.to_string()
    };

    let body_text = format!(
        r#"Hello {},

{} replied to your thread:

"{}"

---
{}
---

View the reply: {}

To stop receiving these emails, update your notification preferences in your account settings.

---
Ruforo Forum
"#,
        recipient_username, replier_username, thread_title, preview, post_link
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>New Reply to Your Thread</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>New Reply to Your Thread</h2>
        <p>Hello <strong>{}</strong>,</p>
        <p><strong>{}</strong> replied to your thread:</p>
        <h3 style="color: #007bff;">{}</h3>
        <div style="background: #f8f9fa; border-left: 4px solid #28a745; padding: 15px; margin: 20px 0;">
            <p style="margin: 0; white-space: pre-wrap;">{}</p>
        </div>
        <p style="margin: 30px 0;">
            <a href="{}"
               style="background-color: #28a745; color: white; padding: 12px 24px;
                      text-decoration: none; border-radius: 4px; display: inline-block;">
                View Reply
            </a>
        </p>
        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ddd;">
        <p style="color: #666; font-size: 0.9em;">
            To stop receiving these emails, update your notification preferences in your account settings.
        </p>
    </div>
</body>
</html>"#,
        recipient_username, replier_username, thread_title, preview, post_link
    );

    let subject = format!("Re: {}", thread_title);
    send_email(to, &subject, &body_text, Some(&body_html)).await
}

/// Send a quote notification email
pub async fn send_quote_email(
    to: &str,
    recipient_username: &str,
    quoter_username: &str,
    thread_title: &str,
    thread_id: i32,
    post_id: i32,
    post_preview: &str,
    base_url: &str,
) -> EmailResult<()> {
    let post_link = format!("{}/threads/{}#post-{}", base_url, thread_id, post_id);

    // Truncate preview to 500 chars
    let preview = if post_preview.len() > 500 {
        format!("{}...", &post_preview[..500])
    } else {
        post_preview.to_string()
    };

    let body_text = format!(
        r#"Hello {},

{} quoted you in a post:

"{}"

---
{}
---

View the post: {}

To stop receiving these emails, update your notification preferences in your account settings.

---
Ruforo Forum
"#,
        recipient_username, quoter_username, thread_title, preview, post_link
    );

    let body_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>You were quoted</title>
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2>You were quoted</h2>
        <p>Hello <strong>{}</strong>,</p>
        <p><strong>{}</strong> quoted your post in:</p>
        <h3 style="color: #007bff;">{}</h3>
        <div style="background: #f8f9fa; border-left: 4px solid #6f42c1; padding: 15px; margin: 20px 0;">
            <p style="margin: 0; white-space: pre-wrap;">{}</p>
        </div>
        <p style="margin: 30px 0;">
            <a href="{}"
               style="background-color: #6f42c1; color: white; padding: 12px 24px;
                      text-decoration: none; border-radius: 4px; display: inline-block;">
                View Post
            </a>
        </p>
        <hr style="margin: 30px 0; border: none; border-top: 1px solid #ddd;">
        <p style="color: #666; font-size: 0.9em;">
            To stop receiving these emails, update your notification preferences in your account settings.
        </p>
    </div>
</body>
</html>"#,
        recipient_username, quoter_username, thread_title, preview, post_link
    );

    let subject = format!("{} quoted you in: {}", quoter_username, thread_title);
    send_email(to, &subject, &body_text, Some(&body_html)).await
}
