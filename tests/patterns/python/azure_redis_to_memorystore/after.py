import redis

r = redis.Redis(host='10.0.0.3', port=6379)


def cache_get(k: str):
    return r.get(k)


def cache_set(k: str, v: str, ttl=3600):
    r.setex(k, ttl, v)
