"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class ElastiCacheManager:
    """Manages AWS ElastiCache clusters"""
    
    def __init__(self, region_name='us-east-1'):
        self.elasticache_client = boto3.client('elasticache', region_name=region_name)
    
    def create_cache_cluster(self, cluster_id, node_type, num_cache_nodes=1, engine='redis'):
        """Create an ElastiCache cluster"""
        try:
            response = self.elasticache_client.create_cache_cluster(
                CacheClusterId=cluster_id,
                NodeType=node_type,
                NumCacheNodes=num_cache_nodes,
                Engine=engine
            )
            print(f"ElastiCache cluster {cluster_id} creation initiated")
            return response
        except ClientError as e:
            print(f"Error creating cache cluster: {e}")
            return None
    
    def describe_cache_clusters(self, cluster_id=None):
        """Describe ElastiCache clusters"""
        try:
            params = {}
            if cluster_id:
                params['CacheClusterId'] = cluster_id
            
            response = self.elasticache_client.describe_cache_clusters(**params)
            return response.get('CacheClusters', [])
        except ClientError as e:
            print(f"Error describing cache clusters: {e}")
            return []
    
    def delete_cache_cluster(self, cluster_id):
        """Delete an ElastiCache cluster"""
        try:
            self.elasticache_client.delete_cache_cluster(CacheClusterId=cluster_id)
            print(f"ElastiCache cluster {cluster_id} deletion initiated")
            return True
        except ClientError as e:
            print(f"Error deleting cache cluster: {e}")
            return False
