#include <cstdint>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <string>

std::string read_stdin() {
  std::ostringstream buffer;
  buffer << std::cin.rdbuf();
  return buffer.str();
}

std::string extract_field(const std::string &json, const std::string &key) {
  std::string pattern = "\"" + key + "\"";
  size_t key_pos = json.find(pattern);
  if (key_pos == std::string::npos) {
    return "";
  }
  size_t colon = json.find(':', key_pos + pattern.size());
  if (colon == std::string::npos) {
    return "";
  }
  size_t quote_start = json.find('"', colon + 1);
  if (quote_start == std::string::npos) {
    return "";
  }
  size_t quote_end = json.find('"', quote_start + 1);
  if (quote_end == std::string::npos) {
    return "";
  }
  return json.substr(quote_start + 1, quote_end - quote_start - 1);
}

uint64_t fnv1a64(const std::string &input) {
  uint64_t hash = 0xcbf29ce484222325ULL;
  for (unsigned char byte : input) {
    hash ^= static_cast<uint64_t>(byte);
    hash *= 0x100000001b3ULL;
  }
  return hash;
}

std::string hex_u64(uint64_t value) {
  std::ostringstream out;
  out << std::hex << std::setfill('0') << std::setw(16) << value;
  return out.str();
}

std::string json_escape(const std::string &value) {
  std::string out;
  out.reserve(value.size());
  for (char ch : value) {
    switch (ch) {
      case '\\': out += "\\\\"; break;
      case '"': out += "\\\""; break;
      case '\n': out += "\\n"; break;
      case '\r': out += "\\r"; break;
      case '\t': out += "\\t"; break;
      default: out += ch; break;
    }
  }
  return out;
}

int main() {
  std::string input = read_stdin();
  if (input.empty()) {
    std::cerr << "empty input";
    return 1;
  }

  std::string job_type = extract_field(input, "job_type");
  std::string payload_b64 = extract_field(input, "payload_b64");

  if (job_type.empty() || payload_b64.empty()) {
    std::cerr << "missing fields";
    return 1;
  }

  std::string result = "processed|" + job_type + "|" + payload_b64;
  uint64_t checksum = fnv1a64(result);

  std::cout << "{"
            << "\"result\":\"" << json_escape(result) << "\","
            << "\"checksum\":\"" << hex_u64(checksum) << "\","
            << "\"algorithm\":\"fnv1a-64\""
            << "}";
  return 0;
}
