import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  scenarios: {
    public_catalog: {
      executor: 'constant-vus',
      vus: 20,
      duration: '20s',
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<500'],
  },
};

const baseUrl = __ENV.BASE_URL || 'http://127.0.0.1:8080';

export default function () {
  const health = http.get(`${baseUrl}/api/health`);
  check(health, { 'health is 200': (response) => response.status === 200 });
  const products = http.get(`${baseUrl}/api/products?limit=24`);
  check(products, { 'catalog is 200': (response) => response.status === 200 });
  sleep(0.2);
}
