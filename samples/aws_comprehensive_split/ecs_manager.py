"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class ECSManager:
    """Manages AWS ECS clusters and tasks"""
    
    def __init__(self, region_name='us-east-1'):
        self.ecs_client = boto3.client('ecs', region_name=region_name)
    
    def create_cluster(self, cluster_name):
        """Create an ECS cluster"""
        try:
            response = self.ecs_client.create_cluster(clusterName=cluster_name)
            print(f"ECS cluster {cluster_name} created")
            return response
        except ClientError as e:
            print(f"Error creating cluster: {e}")
            return None
    
    def register_task_definition(self, family, container_definitions, cpu='256', memory='512'):
        """Register an ECS task definition"""
        try:
            response = self.ecs_client.register_task_definition(
                family=family,
                containerDefinitions=container_definitions,
                cpu=cpu,
                memory=memory,
                requiresCompatibilities=['FARGATE'],
                networkMode='awsvpc'
            )
            print(f"Task definition {family} registered")
            return response
        except ClientError as e:
            print(f"Error registering task definition: {e}")
            return None
    
    def run_task(self, cluster_name, task_definition, subnets, security_groups):
        """Run an ECS task"""
        try:
            response = self.ecs_client.run_task(
                cluster=cluster_name,
                taskDefinition=task_definition,
                launchType='FARGATE',
                networkConfiguration={
                    'awsvpcConfiguration': {
                        'subnets': subnets,
                        'securityGroups': security_groups,
                        'assignPublicIp': 'ENABLED'
                    }
                }
            )
            print(f"Task started in cluster {cluster_name}")
            return response
        except ClientError as e:
            print(f"Error running task: {e}")
            return None
    
    def list_tasks(self, cluster_name):
        """List tasks in an ECS cluster"""
        try:
            response = self.ecs_client.list_tasks(cluster=cluster_name)
            return response.get('taskArns', [])
        except ClientError as e:
            print(f"Error listing tasks: {e}")
            return []
