"""Route 53 — split from aws_comprehensive_example.py."""
import boto3
import uuid
from botocore.exceptions import ClientError

class Route53Manager:
    """Manages AWS Route 53 DNS records"""
    
    def __init__(self):
        self.route53_client = boto3.client('route53')
    
    def create_hosted_zone(self, name, caller_reference=None):
        """Create a Route 53 hosted zone"""
        try:
            if not caller_reference:
                caller_reference = str(uuid.uuid4())
            
            response = self.route53_client.create_hosted_zone(
                Name=name,
                CallerReference=caller_reference
            )
            zone_id = response['HostedZone']['Id'].split('/')[-1]
            print(f"Hosted zone {name} created with ID: {zone_id}")
            return response
        except ClientError as e:
            print(f"Error creating hosted zone: {e}")
            return None
    
    def create_record(self, hosted_zone_id, name, record_type, value, ttl=300):
        """Create a DNS record"""
        try:
            response = self.route53_client.change_resource_record_sets(
                HostedZoneId=hosted_zone_id,
                ChangeBatch={
                    'Changes': [{
                        'Action': 'CREATE',
                        'ResourceRecordSet': {
                            'Name': name,
                            'Type': record_type,
                            'TTL': ttl,
                            'ResourceRecords': [{'Value': value}]
                        }
                    }]
                }
            )
            print(f"DNS record {name} created")
            return response
        except ClientError as e:
            print(f"Error creating DNS record: {e}")
            return None
    
    def list_hosted_zones(self):
        """List all hosted zones"""
        try:
            response = self.route53_client.list_hosted_zones()
            return response.get('HostedZones', [])
        except ClientError as e:
            print(f"Error listing hosted zones: {e}")
            return []
