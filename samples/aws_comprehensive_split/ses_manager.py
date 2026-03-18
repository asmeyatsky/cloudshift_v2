"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class SESManager:
    """Manages AWS SES email sending"""
    
    def __init__(self, region_name='us-east-1'):
        self.ses_client = boto3.client('ses', region_name=region_name)
    
    def verify_email_address(self, email_address):
        """Verify an email address for SES"""
        try:
            self.ses_client.verify_email_identity(EmailAddress=email_address)
            print(f"Verification email sent to {email_address}")
            return True
        except ClientError as e:
            print(f"Error verifying email: {e}")
            return False
    
    def send_email(self, source, destination, subject, body_text, body_html=None):
        """Send an email via SES"""
        try:
            params = {
                'Source': source,
                'Destination': {'ToAddresses': [destination]},
                'Message': {
                    'Subject': {'Data': subject},
                    'Body': {'Text': {'Data': body_text}}
                }
            }
            if body_html:
                params['Message']['Body']['Html'] = {'Data': body_html}
            
            response = self.ses_client.send_email(**params)
            print(f"Email sent successfully. Message ID: {response['MessageId']}")
            return response
        except ClientError as e:
            print(f"Error sending email: {e}")
            return None
    
    def list_verified_emails(self):
        """List verified email addresses"""
        try:
            response = self.ses_client.list_verified_email_addresses()
            return response.get('VerifiedEmailAddresses', [])
        except ClientError as e:
            print(f"Error listing verified emails: {e}")
            return []
