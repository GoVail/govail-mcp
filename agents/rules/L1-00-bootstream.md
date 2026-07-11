# L1-00-bootstream

**MANDATORY**: 이 파일은 GoVail MCP 저장소에서 에이전트가 가장 먼저 읽어야 하는 부트스트림 규칙이다.

## 1. 필수 로딩 순서

작업 시작 시 아래 순서를 따른다.

1. `agents/rules/L1-00-bootstream.md`
2. `agents/rules/rules.md`
3. 작업 성격에 맞는 설계 문서와 README

## 2. 실행 환경

- M1 Max는 코드 편집 전용이다.
- 빌드, 서비스 구동, Docker Compose, 통합 검증은 맥미니 구동 환경에서 수행한다.
- LLM 호출은 직접 모델 서버를 호출하지 않고 프로젝트 서버의 OpenAI 호환 게이트웨이 또는 LiteLLM 경로로 주입한다.

## 3. 충돌 우선순위

규칙이 충돌하면 아래 순서를 적용한다.

1. 사용자의 최신 명시 지시
2. 로컬 도메인 룰 `agents/rules/rules.md`
3. 이 부트스트림 룰
4. 일반 문서와 과거 인계 내용
