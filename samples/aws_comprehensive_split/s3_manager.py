"""S3 — split from aws_comprehensive_example.py (transform this file alone in CloudShift)."""
import boto3
import os
from botocore.exceptions import ClientError

class S3Manager:
    """Manages S3 bucket operations"""
    
    def __init__(self, region_name='us-east-1'):
        self.s3_client = boto3.client('s3', region_name=region_name)
        self.s3_resource = boto3.resource('s3', region_name=region_name)
        self.bucket_name = os.environ.get('S3_BUCKET_NAME', 'my-app-bucket')
    
    def upload_file(self, local_file_path, s3_key):
        """Upload a file to S3 bucket"""
        try:
            self.s3_client.upload_file(
                local_file_path,
                self.bucket_name,
                s3_key,
                ExtraArgs={'ContentType': 'application/json'}
            )
            print(f"Successfully uploaded {local_file_path} to s3://{self.bucket_name}/{s3_key}")
            return True
        except ClientError as e:
            print(f"Error uploading file: {e}")
            return False
    
    def download_file(self, s3_key, local_file_path):
        """Download a file from S3 bucket"""
        try:
            self.s3_resource.Bucket(self.bucket_name).download_file(s3_key, local_file_path)
            print(f"Successfully downloaded s3://{self.bucket_name}/{s3_key} to {local_file_path}")
            return True
        except ClientError as e:
            print(f"Error downloading file: {e}")
            return False
    
    def list_objects(self, prefix=''):
        """List all objects in S3 bucket with given prefix"""
        try:
            response = self.s3_client.list_objects_v2(
                Bucket=self.bucket_name,
                Prefix=prefix
            )
            return response.get('Contents', [])
        except ClientError as e:
            print(f"Error listing objects: {e}")
            return []
    
    def delete_object(self, s3_key):
        """Delete an object from S3 bucket"""
        try:
            self.s3_client.delete_object(Bucket=self.bucket_name, Key=s3_key)
            print(f"Successfully deleted s3://{self.bucket_name}/{s3_key}")
            return True
        except ClientError as e:
            print(f"Error deleting object: {e}")
            return False
    
    def generate_presigned_url(self, s3_key, expiration=3600):
        """Generate a presigned URL for temporary access"""
        try:
            url = self.s3_client.generate_presigned_url(
                'get_object',
                Params={'Bucket': self.bucket_name, 'Key': s3_key},
                ExpiresIn=expiration
            )
            return url
        except ClientError as e:
            print(f"Error generating presigned URL: {e}")
            return None
