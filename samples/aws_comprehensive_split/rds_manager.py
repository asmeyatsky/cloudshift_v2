"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class RDSManager:
    """Manages RDS database instances"""
    
    def __init__(self, region_name='us-east-1'):
        self.rds_client = boto3.client('rds', region_name=region_name)
    
    def create_database_instance(self, db_instance_identifier, db_name, master_username, master_password, db_instance_class):
        """Create an RDS database instance"""
        try:
            response = self.rds_client.create_db_instance(
                DBInstanceIdentifier=db_instance_identifier,
                DBName=db_name,
                DBInstanceClass=db_instance_class,
                Engine='postgres',
                MasterUsername=master_username,
                MasterUserPassword=master_password,
                AllocatedStorage=20,
                VpcSecurityGroupIds=[],
                BackupRetentionPeriod=7,
                MultiAZ=False,
                PubliclyAccessible=False
            )
            print(f"RDS instance {db_instance_identifier} creation initiated")
            return response
        except ClientError as e:
            print(f"Error creating RDS instance: {e}")
            return None
    
    def describe_db_instances(self):
        """Describe all RDS instances"""
        try:
            response = self.rds_client.describe_db_instances()
            return response.get('DBInstances', [])
        except ClientError as e:
            print(f"Error describing RDS instances: {e}")
            return []
    
    def delete_db_instance(self, db_instance_identifier, skip_final_snapshot=True):
        """Delete an RDS database instance"""
        try:
            self.rds_client.delete_db_instance(
                DBInstanceIdentifier=db_instance_identifier,
                SkipFinalSnapshot=skip_final_snapshot
            )
            print(f"RDS instance {db_instance_identifier} deletion initiated")
            return True
        except ClientError as e:
            print(f"Error deleting RDS instance: {e}")
            return False
