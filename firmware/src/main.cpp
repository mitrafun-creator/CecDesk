#include <Arduino.h>
#include <CEC_Device.h>
#include <EEPROM.h>

#if defined(ARDUINO_ARCH_ESP32)
#include "tusb.h"
#include <Update.h>
#define CEC_PIN 4              // GPIO4 на ESP32-S3 (Шина HDMI CEC, Pin 13)
#define DEFAULT_PC_PWR_PIN 5   // GPIO5 на ESP32-S3 (Оптопара PWR_BTN материнской платы)
#define DEFAULT_PC_RST_PIN 6   // GPIO6 на ESP32-S3 (Оптопара RESET_BTN материнской платы)
#define DEFAULT_PC_SENSE_PIN 7 // GPIO7 на ESP32-S3 (Вход контроля питания PWR_LED+)
#elif defined(ARDUINO_ARCH_RP2040)
#include <pico/bootrom.h>
#define CEC_PIN 0              // GP0 (Вывод 1) на RP2040-Zero (Шина HDMI CEC, Pin 13)
#define DEFAULT_PC_PWR_PIN 1   // GP1 (Вывод 4) на RP2040-Zero (Оптопара PWR_BTN материнской платы)
#define DEFAULT_PC_RST_PIN 2   // GP2 (Вывод 5) на RP2040-Zero (Оптопара RESET_BTN материнской платы)
#define DEFAULT_PC_SENSE_PIN 3 // GP3 (Вывод 6) на RP2040-Zero (Вход контроля питания PWR_LED+)
#else
#define CEC_PIN 4
#define DEFAULT_PC_PWR_PIN 5
#define DEFAULT_PC_RST_PIN 6
#define DEFAULT_PC_SENSE_PIN 7
#endif

#define EEPROM_MAGIC 0xC6

uint8_t pcPwrPin = DEFAULT_PC_PWR_PIN;
uint8_t pcRstPin = DEFAULT_PC_RST_PIN;
bool wakePcOnTvOn = true;        // Авто-включение ПК через оптопару при включении ТВ
bool standbyTvOnPcOff = false;   // Авто-выключение ТВ при уходе ПК в сон / выключении

// Неблокирующий таймер импульса оптопары
int activeOptoPin = -1;
unsigned long optoPulseEndMs = 0;

// Контроль активности хоста ПК (по USB Serial и пинг-запросам STATE)
unsigned long lastHostSerialMs = 0;
bool wasHostActive = false;
unsigned long lastAutoWakeMs = 0;

void sendReply(const String& msg) {
  Serial.println(msg);
#if defined(ARDUINO_ARCH_ESP32)
  Serial0.println(msg);
#endif
}

void saveSettingsToEeprom() {
  uint8_t wantWake = wakePcOnTvOn ? 1 : 0;
  uint8_t wantStandby = standbyTvOnPcOff ? 1 : 0;
  if (EEPROM.read(0) == EEPROM_MAGIC &&
      EEPROM.read(1) == wantWake &&
      EEPROM.read(2) == wantStandby &&
      EEPROM.read(3) == pcPwrPin) {
    return;
  }
  EEPROM.write(0, EEPROM_MAGIC);
  EEPROM.write(1, wantWake);
  EEPROM.write(2, wantStandby);
  EEPROM.write(3, pcPwrPin);
  EEPROM.commit();
}

void loadSettingsFromEeprom() {
  EEPROM.begin(64);
  if (EEPROM.read(0) == EEPROM_MAGIC) {
    wakePcOnTvOn = (EEPROM.read(1) != 0);
    standbyTvOnPcOff = (EEPROM.read(2) != 0);
    uint8_t savedPin = EEPROM.read(3);
    if (savedPin > 0 && savedPin < 40 && savedPin != CEC_PIN) {
      pcPwrPin = savedPin;
    }
  } else {
    saveSettingsToEeprom();
  }
}

void startOptoPulse(uint8_t pin, unsigned long durationMs) {
  pinMode(pin, OUTPUT);
  digitalWrite(pin, HIGH);
  activeOptoPin = (int)pin;
  optoPulseEndMs = millis() + durationMs;
}

void updateOptoPulse() {
  if (activeOptoPin >= 0 && (long)(millis() - optoPulseEndMs) >= 0) {
    digitalWrite((uint8_t)activeOptoPin, LOW);
    activeOptoPin = -1;
    sendReply("OK:OPTO_PULSE_DONE");
  }
}

bool isPcHostActive() {
  if (lastHostSerialMs > 0 && (millis() - lastHostSerialMs < 12500)) {
    return true;
  }
#if defined(ARDUINO_ARCH_ESP32)
  if (!tud_suspended() && lastHostSerialMs > 0 && (millis() - lastHostSerialMs < 20000)) {
    return true;
  }
#endif
  return false;
}

struct QueuedTx {
  uint8_t target;
  uint8_t data[15];
  uint8_t len;
};

class MyCEC : public CEC_Device {
public:
  uint16_t myPhysicalAddr = 0x3000; // По умолчанию HDMI 3 (3.0.0.0)
  String osdName = "GAMING PC";     // Имя порта по умолчанию
  bool autoReply = true;            // Автоответ на системные опросы ТВ
  bool remoteCaptureEnabled = false;// По умолчанию перехват пульта ВЫКЛЮЧЕН, пока не запущена программа на ПК
  bool nonIntrusiveMode = true;     // Вежливый режим CEC (без принудительного перехвата HDMI при старте)
  bool announcedOnce = false;
  bool txSuccessFlag = false;
  bool txDoneFlag = false;

  QueuedTx txQueue[12];
  int qHead = 0;
  int qTail = 0;
  unsigned long lastRxOrTxEndMs = 0;

  MyCEC(int pin) : CEC_Device(0x3000, pin, pin) {}

  bool isCaptureActive() const {
    return remoteCaptureEnabled && isPcHostActive();
  }

  // Физический адрес HDMI-порта ПК (по умолчанию HDMI 3 - 0x3000)
  uint16_t getReportedPhysicalAddr() const {
    return myPhysicalAddr;
  }

  virtual bool LineState() override {
    return digitalRead(CEC_PIN) == HIGH;
  }

  bool isReadyAndAssigned() const {
    return _primaryState == CEC_IDLE && _logicalAddress != 15;
  }

  int getLogicalAddr() const {
    return _logicalAddress;
  }

  virtual void SetLineState(bool state) override {
    if (state) {
      pinMode(CEC_PIN, INPUT_PULLUP);
    } else {
      pinMode(CEC_PIN, OUTPUT);
      digitalWrite(CEC_PIN, LOW);
    }
    delayMicroseconds(50);
    _lastLineState2 = LineState();
  }

  void Initialize(CEC_DEVICE_TYPE type) {
    pinMode(CEC_PIN, INPUT_PULLUP);
    delay(200);
    CEC_LogicalDevice::Initialize(type);
  }

  virtual void OnReady() override {
    CEC_Device::OnReady();
    MonitorMode = false;
    SetAddress(_logicalAddress);
    char buf[48];
    snprintf(buf, sizeof(buf), "OK:READY_LA_%d", _logicalAddress);
    sendReply(buf);
  }

  virtual void OnTransmitComplete(bool success) override {
    CEC_LogicalDevice::OnTransmitComplete(success);
    txSuccessFlag = success;
    txDoneFlag = true;
    lastRxOrTxEndMs = millis();
  }

  // Безопасная постановка пакета в очередь (БЕЗ вложенного вызова Run() внутри OnReceive!)
  void enqueueTx(uint8_t target, const uint8_t* data, uint8_t len) {
    int nextTail = (qTail + 1) % 12;
    if (nextTail == qHead) return; // Очередь полна
    txQueue[qTail].target = target;
    txQueue[qTail].len = min((uint8_t)15, len);
    memcpy(txQueue[qTail].data, data, txQueue[qTail].len);
    qTail = nextTail;
  }

  // Выполняется только из главного loop() на чистом стеке!
  void pumpTxQueue() {
    if (qHead == qTail) return;
    if (!isReadyAndAssigned() || TransmitPending()) return;
    // Выдерживаем тишину 60 мс после последнего пакета на шине
    if (millis() - lastRxOrTxEndMs < 60) return;

    QueuedTx item = txQueue[qHead];
    qHead = (qHead + 1) % 12;

    char logBuf[96];
    int pos = snprintf(logBuf, sizeof(logBuf), "TX %01X->%01X:", _logicalAddress & 0x0F, item.target & 0x0F);
    for (int i = 0; i < item.len && pos < (int)sizeof(logBuf) - 4; i++) {
      pos += snprintf(logBuf + pos, sizeof(logBuf) - pos, " %02X", item.data[i]);
    }
    sendReply(logBuf);

    bool wasMonitor = MonitorMode;
    if (wasMonitor) {
      MonitorMode = false;
      SetAddress(_logicalAddress);
    }

    txDoneFlag = false;
    TransmitFrame(item.target, item.data, item.len);

    // Ждем фактического завершения передачи кадра (до 450 мс)
    unsigned long t0 = millis();
    while (!txDoneFlag && (millis() - t0 < 450)) {
      Run();
    }

    if (wasMonitor) {
      MonitorMode = true;
      SetAddress(15);
    }
    lastRxOrTxEndMs = millis();
  }

  virtual void OnReceive(int source, int dest, unsigned char* buffer, int count) override {
    lastRxOrTxEndMs = millis();
#if defined(ARDUINO_ARCH_ESP32)
    if (tud_suspended()) {
      tud_remote_wakeup(); // Пробуждение ПК из спящего режима по USB при нажатии кнопок пульта
    }
#endif
    char logBuf[96];
    int pos = snprintf(logBuf, sizeof(logBuf), "RX %01X->%01X:", source & 0x0F, dest & 0x0F);
    for (int i = 0; i < count && pos < (int)sizeof(logBuf) - 4; i++) {
      pos += snprintf(logBuf + pos, sizeof(logBuf) - pos, " %02X", buffer[i]);
    }
    sendReply(logBuf);

    if (count == 0 || !autoReply) return;
    uint8_t opcode = buffer[0];

    // Детекция включения телевизора или переключения на вход ПК для пробуждения ПК через оптопару
    bool isTvPowerOrWakeEvent = false;
    if (opcode == 0x85 && dest == 0x0F) {
      isTvPowerOrWakeEvent = true;
    } else if (opcode == 0x84 && (source & 0x0F) == 0x00) {
      isTvPowerOrWakeEvent = true;
    } else if (opcode == 0x90 && count >= 2 && buffer[1] == 0x00 && (source & 0x0F) == 0x00) {
      isTvPowerOrWakeEvent = true;
    } else if ((opcode == 0x80 || opcode == 0x86) && count >= 3) {
      uint16_t targetPa = (opcode == 0x80 && count >= 5)
                        ? ((buffer[3] << 8) | buffer[4])
                        : ((buffer[1] << 8) | buffer[2]);
      if (targetPa == myPhysicalAddr) {
        isTvPowerOrWakeEvent = true;
      }
    } else if (opcode == 0x44 && count >= 2 && (buffer[1] == 0x40 || buffer[1] == 0x6D)) {
      isTvPowerOrWakeEvent = true;
    }

    if (isTvPowerOrWakeEvent && wakePcOnTvOn && !isPcHostActive()) {
      if (lastAutoWakeMs == 0 || (millis() - lastAutoWakeMs > 15000)) {
        lastAutoWakeMs = millis();
        startOptoPulse(pcPwrPin, 350);
        sendReply("OK:AUTO_OPTO_WAKE_PC");
      }
    }

    if (opcode == 0x83 && (dest == _logicalAddress || dest == 0x0F)) {
      uint16_t repPa = getReportedPhysicalAddr();
      uint8_t frame[4] = {0x84, (uint8_t)(repPa >> 8), (uint8_t)(repPa & 0xFF), 0x04};
      enqueueTx(0x0F, frame, 4);
    }
    else if (opcode == 0x8C && (dest == _logicalAddress || dest == 0x0F)) {
      // Стандартный HDMI IEEE OUI (00-0C-03) — не вызывает у Philips проприетарный спам 0x89!
      uint8_t frame[4] = {0x87, 0x00, 0x0C, 0x03};
      enqueueTx(0x0F, frame, 4);
    }
    else if (opcode == 0x9F && dest == _logicalAddress) {
      uint8_t frame[2] = {0x9E, 0x04}; // CEC 1.3a
      enqueueTx(source, frame, 2);
    }
    else if (opcode == 0x46 && dest == _logicalAddress) {
      enqueueOsdName(source);
    }
    else if (opcode == 0x8F && dest == _logicalAddress) {
      // Всегда отвечаем Power ON (0x00), чтобы телевизор держал HDMI-аудиопоток активным
      uint8_t frame[2] = {0x90, 0x00};
      enqueueTx(source, frame, 2);
    }
    else if ((opcode == 0x8D || opcode == 0x8E) && dest == _logicalAddress) {
      uint8_t frame[2] = {0x8E, (uint8_t)(isCaptureActive() ? 0x00 : 0x01)}; // 0x00 = Menu Active, 0x01 = Menu Inactive
      enqueueTx(source, frame, 2);
    }
    else if ((opcode == 0x1A || opcode == 0x41 || opcode == 0x42) && dest == _logicalAddress) {
      uint8_t frame[2] = {0x1B, (uint8_t)(isCaptureActive() ? 0x11 : 0x20)}; // 0x11 = Play, 0x20 = Stop
      enqueueTx(source, frame, 2);
    }
    else if (opcode == 0x85 && dest == 0x0F) {
      if (isCaptureActive()) {
        uint8_t frame[3] = {0x82, (uint8_t)(myPhysicalAddr >> 8), (uint8_t)(myPhysicalAddr & 0xFF)};
        enqueueTx(0x0F, frame, 3);
        uint8_t menuFrame[2] = {0x8E, 0x00};
        enqueueTx(0x00, menuFrame, 2);
      }
    }
    else if ((opcode == 0x80 || opcode == 0x86) && count >= 3) {
      uint16_t targetPa = (opcode == 0x80 && count >= 5)
                        ? ((buffer[3] << 8) | buffer[4])
                        : ((buffer[1] << 8) | buffer[2]);
      if (targetPa == myPhysicalAddr && isCaptureActive()) {
        uint8_t frame[3] = {0x82, (uint8_t)(myPhysicalAddr >> 8), (uint8_t)(myPhysicalAddr & 0xFF)};
        enqueueTx(0x0F, frame, 3);
        uint8_t menuFrame[2] = {0x8E, 0x00};
        enqueueTx(0x00, menuFrame, 2);
      }
    }
    else if (opcode == 0x44 && dest == _logicalAddress) {
      if (!isCaptureActive()) {
        // Полный отказ от перехвата пульта без сброса физического адреса:
        // Отвечаем Feature Abort (Refused), освобождая пульт для собственных функций ТВ
        uint8_t abortFrame[3] = {0x00, 0x44, 0x04};
        enqueueTx(source, abortFrame, 3);
        uint8_t menuFrame[2] = {0x8E, 0x01};
        enqueueTx(source, menuFrame, 2);
      }
    }
    else if (opcode == 0x45 && dest == _logicalAddress) {
      if (!isCaptureActive()) {
        uint8_t abortFrame[3] = {0x00, 0x45, 0x04};
        enqueueTx(source, abortFrame, 3);
      }
    }
  }

  void enqueueOsdName(uint8_t target = 0x00) {
    uint8_t frame[15];
    frame[0] = 0x47;
    size_t len = min((size_t)14, osdName.length());
    memcpy(&frame[1], osdName.c_str(), len);
    enqueueTx(target, frame, len + 1);
  }

  void powerOnTV() {
    uint8_t frame[1] = {0x04};
    enqueueTx(0x00, frame, 1);
  }

  void standbyTV() {
    uint8_t frame[1] = {0x36};
    enqueueTx(0x0F, frame, 1);
  }

  void setActiveSource() {
    powerOnTV();
    uint16_t repPa = getReportedPhysicalAddr();
    uint8_t reportPa[4] = {0x84, (uint8_t)(repPa >> 8), (uint8_t)(repPa & 0xFF), 0x04};
    enqueueTx(0x0F, reportPa, 4);
    uint8_t activeSrc[3] = {0x82, (uint8_t)(myPhysicalAddr >> 8), (uint8_t)(myPhysicalAddr & 0xFF)};
    enqueueTx(0x0F, activeSrc, 3);
    if (isCaptureActive()) {
      uint8_t pwrOn[2] = {0x90, 0x00};
      enqueueTx(0x00, pwrOn, 2);
      uint8_t menuFrame[2] = {0x8E, 0x00};
      enqueueTx(0x00, menuFrame, 2);
    }
  }

  void announceDevice() {
    uint16_t repPa = getReportedPhysicalAddr();
    uint8_t reportPa[4] = {0x84, (uint8_t)(repPa >> 8), (uint8_t)(repPa & 0xFF), 0x04};
    enqueueTx(0x0F, reportPa, 4);
    uint8_t vendor[4] = {0x87, 0x00, 0x0C, 0x03};
    enqueueTx(0x0F, vendor, 4);
    enqueueOsdName(0x00);
    uint8_t pwrFrame[2] = {0x90, (uint8_t)(isCaptureActive() ? 0x00 : 0x01)};
    enqueueTx(0x00, pwrFrame, 2);
    if (isCaptureActive() && !nonIntrusiveMode) {
      uint8_t activeSrc[3] = {0x82, (uint8_t)(myPhysicalAddr >> 8), (uint8_t)(myPhysicalAddr & 0xFF)};
      enqueueTx(0x0F, activeSrc, 3);
    }
  }

  void enableRemoteCapture() {
    remoteCaptureEnabled = true;
    MonitorMode = false;
    SetAddress(_logicalAddress);
    // Активируем Профиль 1 (С перехватом пульта): привязываем LA 4 к реальному HDMI-порту ПК (myPhysicalAddr)
    uint8_t reportPa[4] = {0x84, (uint8_t)(myPhysicalAddr >> 8), (uint8_t)(myPhysicalAddr & 0xFF), 0x04};
    enqueueTx(0x0F, reportPa, 4);
    uint8_t pwrOn[2] = {0x90, 0x00};
    enqueueTx(0x00, pwrOn, 2);
    enqueueOsdName(0x00);
    // Отправляем Active Source (0x82) и Deck Play (0x1B 0x11), чтобы ТВ сразу включил пересылку кнопок пульта
    uint8_t activeSrc[3] = {0x82, (uint8_t)(myPhysicalAddr >> 8), (uint8_t)(myPhysicalAddr & 0xFF)};
    enqueueTx(0x0F, activeSrc, 3);
    uint8_t deckPlay[2] = {0x1B, 0x11};
    enqueueTx(0x00, deckPlay, 2);
    uint8_t menuFrame[2] = {0x8E, 0x00};
    enqueueTx(0x00, menuFrame, 2);
  }

  void disableRemoteCapture() {
    remoteCaptureEnabled = false;
    MonitorMode = false;
    SetAddress(_logicalAddress);
    // При выключении перехвата пульта сохраняем реальный физический адрес HDMI (3000) и статус питания ON (0x90 0x00),
    // но переводим статус меню в неактивное (0x8E 0x01) и останавливаем деку (0x1B 0x20).
    // Телевизор полностью освобождает пульт для собственных функций, НЕ отключая звук и видеосигнал HDMI!
    uint8_t reportPa[4] = {0x84, (uint8_t)(myPhysicalAddr >> 8), (uint8_t)(myPhysicalAddr & 0xFF), 0x04};
    enqueueTx(0x0F, reportPa, 4);
    uint8_t pwrOn[2] = {0x90, 0x00};
    enqueueTx(0x00, pwrOn, 2);
    uint8_t deckStop[2] = {0x1B, 0x20};
    enqueueTx(0x00, deckStop, 2);
    uint8_t menuFrame[2] = {0x8E, 0x01};
    enqueueTx(0x00, menuFrame, 2);
  }

  void sendKey(uint8_t target, uint8_t keyCode) {
    uint8_t pressFrame[2] = {0x44, keyCode};
    enqueueTx(target, pressFrame, 2);
    uint8_t releaseFrame[1] = {0x45};
    enqueueTx(target, releaseFrame, 1);
  }
};

MyCEC* cec = nullptr;
String inputCmd = "";
String inputCmd0 = "";

void processCommand(String cmd) {
  cmd.trim();
  if (cmd.length() == 0 || !cec) return;

  lastHostSerialMs = millis();
  wasHostActive = true;

  if (cmd == "ON") {
    cec->powerOnTV();
    sendReply("OK:TV_ON");
  } 
  else if (cmd == "OFF") {
    cec->standbyTV();
    sendReply("OK:TV_OFF");
  } 
  else if (cmd == "ACTIVE") {
    cec->setActiveSource();
    sendReply("OK:ACTIVE_SOURCE");
  } 
  else if (cmd == "ANNOUNCE") {
    cec->announceDevice();
    sendReply("OK:ANNOUNCED");
  }
  else if (cmd == "REMOTE_ON") {
    cec->enableRemoteCapture();
    sendReply("OK:REMOTE_CAPTURE_ON");
  }
  else if (cmd == "REMOTE_OFF") {
    cec->disableRemoteCapture();
    sendReply("OK:REMOTE_CAPTURE_OFF");
  }
  else if (cmd == "PC_PWR_PULSE" || cmd == "OPTO_PULSE" || cmd == "PC_WAKE") {
    startOptoPulse(pcPwrPin, 350);
    sendReply("OK:PC_PWR_PULSE_350MS");
  }
  else if (cmd == "PC_PWR_HOLD" || cmd == "OPTO_HOLD") {
    startOptoPulse(pcPwrPin, 5000);
    sendReply("OK:PC_PWR_HOLD_5000MS");
  }
  else if (cmd == "PC_RESET_PULSE" || cmd == "OPTO_RESET") {
    startOptoPulse(pcRstPin, 350);
    sendReply("OK:PC_RESET_PULSE_350MS");
  }
  else if (cmd.startsWith("PC_WAKE_ON_TV ")) {
    int val = cmd.substring(14).toInt();
    wakePcOnTvOn = (val != 0);
    saveSettingsToEeprom();
    sendReply(wakePcOnTvOn ? "OK:PC_WAKE_ON_TV_1" : "OK:PC_WAKE_ON_TV_0");
  }
  else if (cmd.startsWith("PC_OFF_TV_STANDBY ")) {
    int val = cmd.substring(18).toInt();
    standbyTvOnPcOff = (val != 0);
    saveSettingsToEeprom();
    sendReply(standbyTvOnPcOff ? "OK:PC_OFF_TV_STANDBY_1" : "OK:PC_OFF_TV_STANDBY_0");
  }
  else if (cmd.startsWith("POLITE ")) {
    int val = cmd.substring(7).toInt();
    cec->nonIntrusiveMode = (val != 0);
    sendReply(cec->nonIntrusiveMode ? "OK:POLITE_1" : "OK:POLITE_0");
  }
  else if (cmd.startsWith("PC_PWR_PIN ")) {
    int newPin = cmd.substring(11).toInt();
    if (newPin > 0 && newPin < 40 && newPin != CEC_PIN) {
      pinMode(pcPwrPin, OUTPUT);
      digitalWrite(pcPwrPin, LOW);
      pcPwrPin = (uint8_t)newPin;
      pinMode(pcPwrPin, OUTPUT);
      digitalWrite(pcPwrPin, LOW);
      saveSettingsToEeprom();
      char buf[32];
      snprintf(buf, sizeof(buf), "OK:PC_PWR_PIN_%d", pcPwrPin);
      sendReply(buf);
    } else {
      sendReply("ERR:INVALID_PIN");
    }
  }
  else if (cmd == "INFO" || cmd == "VERSION") {
    #if defined(CONFIG_IDF_TARGET_ESP32S3) || defined(ARDUINO_ESP32S3_DEV) || defined(ARDUINO_ARCH_ESP32)
      sendReply("OK:INFO_HW_ESP32-S3_FW_1.1.0");
    #elif defined(ARDUINO_ARCH_RP2040)
      sendReply("OK:INFO_HW_RP2040-Zero_FW_1.1.0");
    #else
      sendReply("OK:INFO_HW_CEC-BRIDGE_FW_1.1.0");
    #endif
  }
  else if (cmd == "BOOTLOADER") {
    sendReply("OK:BOOTLOADER");
    delay(60);
    #if defined(ARDUINO_ARCH_RP2040)
      reset_usb_boot(0, 0);
    #elif defined(ARDUINO_ARCH_ESP32)
      ESP.restart();
    #endif
  }
#if defined(ARDUINO_ARCH_ESP32)
  else if (cmd.startsWith("OTA_BEGIN ")) {
    size_t fwSize = (size_t)cmd.substring(10).toInt();
    if (fwSize > 0 && Update.begin(fwSize)) {
      sendReply("OK:OTA_BEGIN");
    } else {
      sendReply("ERR:OTA_BEGIN");
    }
  }
  else if (cmd.startsWith("OTA_CHUNK ")) {
    String hexStr = cmd.substring(10);
    hexStr.trim();
    int hexLen = hexStr.length();
    uint8_t chunkBuf[256];
    int byteCount = 0;
    for (int i = 0; i + 1 < hexLen && byteCount < 256; i += 2) {
      char pair[3] = { hexStr[i], hexStr[i + 1], 0 };
      chunkBuf[byteCount++] = (uint8_t)strtol(pair, NULL, 16);
    }
    if (byteCount > 0 && Update.write(chunkBuf, byteCount) == (size_t)byteCount) {
      sendReply("OK:OTA_CHUNK");
    } else {
      sendReply("ERR:OTA_CHUNK");
    }
  }
  else if (cmd == "OTA_END") {
    if (Update.end(true)) {
      sendReply("OK:OTA_END");
      delay(100);
      ESP.restart();
    } else {
      sendReply("ERR:OTA_END");
    }
  }
#endif
  else if (cmd == "STATE") {
    pinMode(CEC_PIN, INPUT_PULLDOWN);
    delayMicroseconds(200);
    bool extPullup = (digitalRead(CEC_PIN) == HIGH);
    pinMode(CEC_PIN, INPUT_PULLUP);
    delayMicroseconds(50);
    char buf[140];
    snprintf(buf, sizeof(buf), "OK:LINE_%s EXT_PULLUP_%s PA_%04X LA_%d OPTO_PIN_%d WAKE_PC_%d NAME_%s",
             cec->LineState() ? "HIGH" : "LOW",
             extPullup ? "YES(TV_CONNECTED)" : "NO(FLOATING)",
             cec->myPhysicalAddr, cec->getLogicalAddr(),
             (int)pcPwrPin, wakePcOnTvOn ? 1 : 0,
             cec->osdName.c_str());
    sendReply(buf);
  }
  else if (cmd == "AUTO ON") {
    cec->autoReply = true;
    sendReply("OK:AUTO_ON");
  }
  else if (cmd == "AUTO OFF") {
    cec->autoReply = false;
    sendReply("OK:AUTO_OFF");
  }
  else if (cmd == "VOL_UP") {
    cec->sendKey(0x00, 0x41);
    sendReply("OK:VOL_UP");
  }
  else if (cmd == "VOL_DOWN") {
    cec->sendKey(0x00, 0x42);
    sendReply("OK:VOL_DOWN");
  }
  else if (cmd == "MUTE") {
    cec->sendKey(0x00, 0x43);
    sendReply("OK:MUTE");
  }
  else if (cmd.startsWith("KEY ")) {
    uint8_t keyCode = (uint8_t)strtol(cmd.substring(4).c_str(), NULL, 16);
    cec->sendKey(0x00, keyCode);
    char buf[32];
    snprintf(buf, sizeof(buf), "OK:KEY_%02X", keyCode);
    sendReply(buf);
  }
  else if (cmd.startsWith("PA ")) {
    cec->myPhysicalAddr = (uint16_t)strtol(cmd.substring(3).c_str(), NULL, 16);
    cec->announceDevice();
    char buf[32];
    snprintf(buf, sizeof(buf), "OK:PA_%04X", cec->myPhysicalAddr);
    sendReply(buf);
  } 
  else if (cmd.startsWith("NAME ")) {
    cec->osdName = cmd.substring(5);
    cec->enqueueOsdName(0x00);
    sendReply("OK:NAME_SET");
  }
  else if (cmd.startsWith("RAW ")) {
    String hexPart = cmd.substring(4);
    hexPart.trim();
    uint8_t buf[16];
    int count = 0;
    int idx = 0;
    while (idx < (int)hexPart.length() && count < 16) {
      int nextSpace = hexPart.indexOf(' ', idx);
      String token = (nextSpace == -1) ? hexPart.substring(idx) : hexPart.substring(idx, nextSpace);
      token.trim();
      if (token.length() > 0) {
        buf[count++] = (uint8_t)strtol(token.c_str(), NULL, 16);
      }
      if (nextSpace == -1) break;
      idx = nextSpace + 1;
    }
    if (count >= 2) {
      cec->enqueueTx(buf[0], &buf[1], count - 1);
      sendReply("OK:RAW_SENT");
    } else {
      sendReply("ERR:RAW_ARGS");
    }
  }
}

void setup() {
  Serial.begin(115200);
#if defined(ARDUINO_ARCH_ESP32)
  Serial0.begin(115200);
#endif
  loadSettingsFromEeprom();
  pinMode(pcPwrPin, OUTPUT);
  digitalWrite(pcPwrPin, LOW);
  pinMode(pcRstPin, OUTPUT);
  digitalWrite(pcRstPin, LOW);
  pinMode(DEFAULT_PC_SENSE_PIN, INPUT_PULLDOWN);

  pinMode(CEC_PIN, INPUT_PULLUP);
  cec = new MyCEC(CEC_PIN);
  cec->Promiscuous = true;
  cec->Initialize(CEC_LogicalDevice::CDT_PLAYBACK_DEVICE);
  sendReply("READY");
}

void loop() {
  updateOptoPulse();

  // Если программа на ПК закрылась или ПК перешел в спящий режим / выключился (прекратились пинги от приложения),
  // автоматически отключаем перехват кнопок пульта, чтобы ТВ управлялся своим пультом напрямую
  if (wasHostActive && !isPcHostActive()) {
    wasHostActive = false;
    if (cec) {
      if (cec->remoteCaptureEnabled) {
        cec->disableRemoteCapture();
        sendReply("OK:AUTO_REMOTE_OFF_HOST_INACTIVE");
      }
      if (standbyTvOnPcOff) {
        cec->standbyTV();
        sendReply("OK:AUTO_TV_STANDBY_ON_PC_OFF");
      }
    }
  }

  if (cec) {
    cec->Run();
    if (!cec->announcedOnce && cec->isReadyAndAssigned()) {
      cec->announcedOnce = true;
      cec->announceDevice();
      sendReply("OK:AUTO_ANNOUNCED");
    }
    cec->pumpTxQueue();
  }

  while (Serial.available()) {
    char c = Serial.read();
    if (c == '\n' || c == '\r') {
      if (inputCmd.length() > 0) {
        processCommand(inputCmd);
        inputCmd = "";
      }
    } else {
      if (inputCmd.length() < 600) {
        inputCmd += c;
      } else {
        inputCmd = "";
      }
    }
  }

#if defined(ARDUINO_ARCH_ESP32)
  while (Serial0.available()) {
    char c = Serial0.read();
    if (c == '\n' || c == '\r') {
      if (inputCmd0.length() > 0) {
        processCommand(inputCmd0);
        inputCmd0 = "";
      }
    } else {
      if (inputCmd0.length() < 600) {
        inputCmd0 += c;
      } else {
        inputCmd0 = "";
      }
    }
  }
#endif
}
