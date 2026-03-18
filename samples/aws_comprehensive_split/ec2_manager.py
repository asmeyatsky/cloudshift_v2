"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class EC2Manager:
    """Manages EC2 instances"""
    
    def __init__(self, region_name='us-east-1'):
        self.ec2_client = boto3.client('ec2', region_name=region_name)
        self.ec2_resource = boto3.resource('ec2', region_name=region_name)
    
    def create_instance(self, image_id, instance_type, key_name, security_group_ids):
        """Launch an EC2 instance"""
        try:
            instances = self.ec2_resource.create_instances(
                ImageId=image_id,
                MinCount=1,
                MaxCount=1,
                InstanceType=instance_type,
                KeyName=key_name,
                SecurityGroupIds=security_group_ids,
                TagSpecifications=[
                    {
                        'ResourceType': 'instance',
                        'Tags': [
                            {'Key': 'Name', 'Value': 'MyAppServer'},
                            {'Key': 'Environment', 'Value': 'Production'}
                        ]
                    }
                ]
            )
            instance = instances[0]
            instance.wait_until_running()
            print(f"Instance {instance.id} launched successfully")
            return instance
        except ClientError as e:
            print(f"Error creating instance: {e}")
            return None
    
    def list_instances(self, filters=None):
        """List EC2 instances"""
        try:
            if filters is None:
                filters = [{'Name': 'instance-state-name', 'Values': ['running']}]
            
            response = self.ec2_client.describe_instances(Filters=filters)
            instances = []
            for reservation in response['Reservations']:
                instances.extend(reservation['Instances'])
            return instances
        except ClientError as e:
            print(f"Error listing instances: {e}")
            return []
    
    def terminate_instance(self, instance_id):
        """Terminate an EC2 instance"""
        try:
            self.ec2_client.terminate_instances(InstanceIds=[instance_id])
            print(f"Instance {instance_id} termination initiated")
            return True
        except ClientError as e:
            print(f"Error terminating instance: {e}")
            return False
    
    def create_security_group(self, group_name, description, vpc_id):
        """Create a security group"""
        try:
            response = self.ec2_client.create_security_group(
                GroupName=group_name,
                Description=description,
                VpcId=vpc_id
            )
            security_group_id = response['GroupId']
            print(f"Security group {group_name} created with ID: {security_group_id}")
            return security_group_id
        except ClientError as e:
            print(f"Error creating security group: {e}")
            return None
    
    def add_security_group_rule(self, group_id, protocol, port, cidr):
        """Add a rule to a security group"""
        try:
            self.ec2_client.authorize_security_group_ingress(
                GroupId=group_id,
                IpProtocol=protocol,
                FromPort=port,
                ToPort=port,
                CidrIp=cidr
            )
            print(f"Rule added to security group {group_id}")
            return True
        except ClientError as e:
            print(f"Error adding security group rule: {e}")
            return False
