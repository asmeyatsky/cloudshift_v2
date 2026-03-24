import boto3

cognito = boto3.client('cognito-idp')
POOL_ID = 'us-east-1_XXXXX'


def get_user(username):
    return cognito.admin_get_user(UserPoolId=POOL_ID, Username=username)


def set_user_password(username, password):
    cognito.admin_set_user_password(
        UserPoolId=POOL_ID, Username=username, Password=password, Permanent=True
    )
