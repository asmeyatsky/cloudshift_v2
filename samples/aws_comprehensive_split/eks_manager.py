"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class EKSManager:
    """Manages AWS EKS clusters"""
    
    def __init__(self, region_name='us-east-1'):
        self.eks_client = boto3.client('eks', region_name=region_name)
    
    def create_cluster(self, cluster_name, role_arn, vpc_config):
        """Create an EKS cluster"""
        try:
            response = self.eks_client.create_cluster(
                name=cluster_name,
                version='1.27',
                roleArn=role_arn,
                resourcesVpcConfig=vpc_config
            )
            print(f"EKS cluster {cluster_name} creation initiated")
            return response
        except ClientError as e:
            print(f"Error creating EKS cluster: {e}")
            return None
    
    def describe_cluster(self, cluster_name):
        """Describe an EKS cluster"""
        try:
            response = self.eks_client.describe_cluster(name=cluster_name)
            return response.get('cluster')
        except ClientError as e:
            print(f"Error describing cluster: {e}")
            return None
    
    def list_clusters(self):
        """List all EKS clusters"""
        try:
            response = self.eks_client.list_clusters()
            return response.get('clusters', [])
        except ClientError as e:
            print(f"Error listing clusters: {e}")
            return []
