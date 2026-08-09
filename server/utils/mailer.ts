import nodemailer from 'nodemailer'

export function useMailer() {
  const config = useRuntimeConfig()

  const transporter = nodemailer.createTransport({
    host: config.smtpHost,
    port: Number(config.smtpPort),
    secure: Number(config.smtpPort) === 465,
    auth: {
      user: config.smtpUser,
      pass: config.smtpPassword
    }
  })

  async function sendEmail({ to, subject, html }: { to: string, subject: string, html: string }) {
    if (!config.smtpHost || !config.smtpUser || !config.smtpPassword) {
      logger.warn('SMTP is not fully configured. Email was not sent.', undefined, { to, subject })
      // We don't throw an error here to prevent blocking the user if SMTP is misconfigured in dev
      return false
    }

    try {
      await transporter.sendMail({
        from: config.smtpFrom || config.smtpUser,
        to,
        subject,
        html
      })
      return true
    } catch (error) {
      logger.error('Failed to send email', error)
      return false
    }
  }

  function getTemplate(title: string, body: string, actionText?: string, actionUrl?: string) {
    // A clean, simple HTML template for emails
    const actionButton = actionText && actionUrl
      ? `<div style="margin-top: 32px;">
           <a href="${actionUrl}" style="background-color: #000; color: #fff; padding: 12px 24px; text-decoration: none; border-radius: 6px; font-weight: 500; display: inline-block;">
             ${actionText}
           </a>
         </div>`
      : ''

    return `
      <!DOCTYPE html>
      <html>
        <head>
          <meta charset="utf-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <style>
            body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; }
            .container { max-width: 600px; margin: 0 auto; padding: 32px 16px; }
            .header { margin-bottom: 24px; }
            .title { font-size: 24px; font-weight: 600; margin: 0; }
            .content { font-size: 16px; }
            .footer { margin-top: 48px; font-size: 14px; color: #666; border-top: 1px solid #eaeaea; padding-top: 16px; }
          </style>
        </head>
        <body>
          <div class="container">
            <div class="header">
              <h1 class="title">${title}</h1>
            </div>
            <div class="content">
              ${body}
              ${actionButton}
            </div>
            <div class="footer">
              If you didn't request this email, you can safely ignore it.
            </div>
          </div>
        </body>
      </html>
    `
  }

  return { sendEmail, getTemplate }
}
