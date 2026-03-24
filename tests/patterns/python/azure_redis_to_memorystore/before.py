import redis

r = redis.Redis(host='mycache.redis.cache.windows.net', port=6380, password=key, ssl=True)


def cache_get(k: str):
    return r.get(k)


def cache_set(k: str, v: str, ttl=3600):
    r.setex(k, ttl, v)
