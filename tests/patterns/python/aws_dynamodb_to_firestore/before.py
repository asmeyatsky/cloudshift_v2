import boto3

dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('users')


def create_user(user_id, name, email):
    table.put_item(
        Item={
            'user_id': user_id,
            'name': name,
            'email': email,
        }
    )


def get_user(user_id):
    response = table.get_item(
        Key={'user_id': user_id}
    )
    return response.get('Item')


def update_user(user_id, name):
    table.update_item(
        Key={'user_id': user_id},
        UpdateExpression='SET #n = :name',
        ExpressionAttributeNames={'#n': 'name'},
        ExpressionAttributeValues={':name': name},
    )


def delete_user(user_id):
    table.delete_item(
        Key={'user_id': user_id}
    )


def query_users_by_email(email):
    response = table.query(
        IndexName='email-index',
        KeyConditionExpression='email = :email',
        ExpressionAttributeValues={':email': email},
    )
    return response['Items']
