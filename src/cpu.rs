use std::{fs::File, io::Read, path::Path};

use anyhow::anyhow;

pub const SCREEN_WIDTH: usize = 640; // 宽
pub const SCREEN_HEIGHT: usize = 320; // 高
const MEMORY_SIZE: usize = 4096; // 内存大小 4k
const REGISTER_SIZE: usize = 16; // 数量 16
const STACK_SIZE: usize = 16; // 堆栈层级
const KEYPAD_SIZE: usize = 16; // 键数量

// chip8字体集
const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

/// OpCode是由两个字节组成的操作码，我们从mem中获取到的mem[i]和mem[i+1]组成一个完整的OpCode。
/// 将这两个字节的操作码拆分，例如OpCode为0xA000，拆分后我们可以获得(0xA, 0x0, 0x0, 0x0)，
/// 这个数据形式方便我们通过match匹配
#[derive(Debug)]
pub struct OpCode {
    first: u8,
    second: u8,
    third: u8,
    fourth: u8,
}

impl OpCode {
    /// 将opcode的所有字段合并为一个u16的opcode数字
    pub fn merged_opcode(&self) -> u16 {
        (self.first as u16) << 12
            | (self.second as u16) << 8
            | (self.third as u16) << 4
            | self.fourth as u16
    }
}

pub struct Emulator {
    opcode: OpCode,            // 操作码
    memory: [u8; MEMORY_SIZE], // 内存

    registers: [u8; REGISTER_SIZE], //  V0～VE
    index_register: u16,            // 索引（i）和程序计数器（pc），从0x000到0xFFF
    program_counter: u16,

    pub gfx: [[u8; SCREEN_WIDTH]; SCREEN_HEIGHT], // 屏幕

    // 两个60hz的定时，当设置在0以上时，它们会倒数到0，每当sound_timer达到0时，系统的蜂鸣器会发出声音
    delay_timer: u8,
    sound_timer: u8,

    stack: [u16; STACK_SIZE], // 系统堆栈
    stack_pointer: usize,     // 堆栈指针

    pub keypad: [bool; KEYPAD_SIZE], // 基于hex的键盘，长度为0x0～0xF，记录键盘状态
}

impl Emulator {
    pub fn new() -> Self {
        let mut chip8 = Emulator {
            opcode: OpCode {
                first: 0,
                second: 0,
                third: 0,
                fourth: 0,
            },
            memory: [0; MEMORY_SIZE],
            registers: [0; REGISTER_SIZE],
            index_register: 0,
            program_counter: 0x200, // chip8解释器本身占用了机器上内存空间的前512个字节，由于这个原因，为原始系统编写的大多数程序都是从内存位置512（0x200）开始的
            gfx: [[0; SCREEN_WIDTH]; SCREEN_HEIGHT],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; STACK_SIZE],
            stack_pointer: 0,
            keypad: [false; KEYPAD_SIZE],
        };
        // 加载字体集到内存前80个字节
        for (index, value) in FONTSET.into_iter().enumerate() {
            chip8.memory[index] = value;
        }
        chip8
    }

    /// 将程序加载到内存中
    pub fn load_rom<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => return Err(anyhow!("打开文件异常: {}", e.to_string())),
        };
        for (index, value) in file.bytes().enumerate() {
            match value {
                Ok(v) => self.memory[index + self.program_counter as usize] = v,
                Err(e) => return Err(anyhow!("读取到错误的字节: {}", e.to_string())),
            }
        }
        Ok(())
    }

    pub fn emulator_cycle(&mut self) {
        // 获取操作码
        self.fetch_opcode();
        // 执行操作码
        self.process_opcode();
        // 更新定时器
        self.update_timer()
    }

    fn fetch_opcode(&mut self) {
        // 根据pc获取操作码，pc是当前程序的位置
        let opcode = (self.memory[self.program_counter as usize] as u16) << 8
            | self.memory[self.program_counter as usize + 1] as u16;
        self.opcode = OpCode {
            first: ((opcode & 0xF000) >> 12) as u8,
            second: ((opcode & 0x0F00) >> 8) as u8,
            third: ((opcode & 0x00F0) >> 4) as u8,
            fourth: (opcode & 0x000F) as u8,
        };
        self.program_counter += 2;
    }

    fn process_opcode(&mut self) {
        self.program_counter += 2; // ？

        // 解码操作码，根据百科上的opcode表定义对应操作码的操作，https://en.wikipedia.org/wiki/CHIP-8#Opcode_table
        match (
            self.opcode.first,
            self.opcode.second,
            self.opcode.third,
            self.opcode.fourth,
        ) {
            (0, 0, 0xE, 0) => self._00e0(),
            (0, 0, 0xE, 0xE) => self._00ee(),
            // 先匹配0x00E0和0x00EE，然后再匹配0x0NNN，因为NNN可能是任何符号，但是0x00E0和0x00EE是特殊操作
            (0, _, _, _) => self._0nnn(),
            (1, _, _, _) => self._1nnn(),
            (2, _, _, _) => self._2nnn(),
            (3, _, _, _) => self._3xnn(),
            (4, _, _, _) => self._4xnn(),
            (5, _, _, 0) => self._5xy0(),
            (6, _, _, _) => self._6xnn(),
            (7, _, _, _) => self._7xnn(),
            (8, _, _, 0) => self._8xy0(),
            (8, _, _, 1) => self._8xy1(),
            (8, _, _, 2) => self._8xy2(),
            (8, _, _, 3) => self._8xy3(),
            (8, _, _, 4) => self._8xy4(),
            (8, _, _, 5) => self._8xy5(),
            (8, _, _, 6) => self._8xy6(),
            (8, _, _, 7) => self._8xy7(),
            (8, _, _, 0xE) => self._8xye(),
            (9, _, _, 0) => self._9xy0(),
            (0xA, _, _, _) => self._annn(),
            (0xB, _, _, _) => self._bnnn(),
            (0xC, _, _, _) => self._cxnn(),
            (0xD, _, _, _) => self._dxyn(),
            (0xE, _, 9, 0xE) => self._ex9e(),
            (0xE, _, 0xA, 1) => self._exa1(),
            (0xF, _, 0, 7) => self._fx07(),
            (0xF, _, 0, 0xA) => self._fx0a(),
            (0xF, _, 1, 5) => self._fx15(),
            (0xF, _, 1, 8) => self._fx18(),
            (0xF, _, 1, 0xE) => self._fx1e(),
            (0xF, _, 2, 9) => self._fx29(),
            (0xF, _, 3, 3) => self._fx33(),
            (0xF, _, 5, 5) => self._fx55(),
            (0xF, _, 6, 5) => self._fx65(),
            _ => {}
        }
    }

    fn update_timer(&mut self) {
        // 更新定时器
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                println!("BEE!");
            }
            self.sound_timer -= 1;
        }
    }
}

/// 定义Chip8相关操作码的操作
/// 根据百科上的 opcode 表定义对应操作码的操作，https://en.wikipedia.org/wiki/CHIP-8#Opcode_table
impl Emulator {
    /// x is 0x_X__
    #[inline]
    fn get_register_vx(&self) -> u8 {
        // Equal `self.registers[((self.opcode.merged_opcode() & 0x0F00)) >> 8 as usize]`
        self.registers[self.opcode.second as usize]
        // self.registers[((self.opcode.merged_opcode() & 0x0F00) >> 8) as usize]
    }

    /// y is 0x__Y_
    #[inline]
    fn get_register_vy(&self) -> u8 {
        // Equal `self.registers[((self.opcode.merged_opcode() & 0x00F0)) >> 4 as usize]`
        self.registers[self.opcode.third as usize]
        // self.registers[((self.opcode.merged_opcode() & 0x00F0) >> 4) as usize]
    }

    ///
    #[inline]
    fn get_mut_register_vx(&mut self) -> &mut u8 {
        &mut self.registers[self.opcode.second as usize]
    }

    /// n is 0x___N
    #[inline]
    fn get_n(&self) -> u8 {
        (self.opcode.merged_opcode() & 0x000F) as u8
    }

    /// nn is 0x__NN
    #[inline]
    fn get_nn(&self) -> u8 {
        (self.opcode.merged_opcode() & 0x00FF) as u8
    }

    /// nnn is 0x_NNN
    #[inline]
    fn get_nnn(&self) -> u16 {
        self.opcode.merged_opcode() & 0x0FFF
    }

    /// 跳过下一个指令
    #[inline]
    fn skip_next_instruction(&mut self) {
        self.program_counter += 2;
    }

    /// 在地址NNN上调用代码例程(routine)(RCA 1802 for COSMAC VIP)，对于大多数rom来说，这个操作不是必须的。
    fn _0nnn(&mut self) {}

    /// 清除屏幕
    /// disp_clear()
    fn _00e0(&mut self) {
        self.gfx = [[0; SCREEN_WIDTH]; SCREEN_HEIGHT];
    }

    /// 从子例程(subroutine)返回。
    /// 当调用子例程时，我们会将当前pc存储到sp位置的stack中，并将栈指针加1，这相当于记录当前帧，
    /// 那么当我们从子例程中返回时，我们需要将栈指针减一以指回原本pc的帧。
    /// return;
    fn _00ee(&mut self) {
        self.stack_pointer -= 1;
        self.program_counter = self.stack[self.stack_pointer] + 2;
    }

    /// 跳转到地址NNN。
    /// 直接将当前程序计数器指向地址
    /// goto NNN;
    fn _1nnn(&mut self) {
        // 1nnn对应opcode的second+third+fourth地址
        self.program_counter = self.get_nnn();
    }

    /// 在NNN处调用子例程(subroutine)
    /// *(0xNNN)()
    fn _2nnn(&mut self) {
        // 因为我们需要临时跳转到地址NNN，这意味着我们应该将程序计数器的当前地址存储在堆栈中。
        // 将程序计数器的值存入栈后，增加栈指针，防止覆盖当前栈。
        self.stack[self.stack_pointer] = self.program_counter - 2;
        self.stack_pointer += 1;
        self.program_counter = self.get_nnn();
    }

    /// 如果VX的值等于NN，则跳过下一条指令（通常下一条指令是跳过一个代码块）
    /// if (Vx == NN)
    fn _3xnn(&mut self) {
        if self.get_register_vx() == self.get_nn() {
            self.skip_next_instruction();
        }
    }

    /// 如果VX的值不等于NN，则跳过下一条指令（通常下一条指令是跳过一个代码块）
    /// if (Vx != NN)
    fn _4xnn(&mut self) {
        if self.get_register_vx() != self.get_nn() {
            self.skip_next_instruction();
        }
    }

    /// 如果VX的值不等于VY，则跳过下一条指令（通常下一条指令是跳过一个代码块）
    /// if (Vx == Vy)
    fn _5xy0(&mut self) {
        if self.get_register_vx() != self.get_register_vy() {
            self.skip_next_instruction();
        }
    }

    /// 将VX的值设置为NN地址的值
    /// Vx = NN
    fn _6xnn(&mut self) {
        *self.get_mut_register_vx() = self.get_nn();
    }

    /// 将VX的值加上NN地址的值（进位标志不变）
    /// Vx += NN
    fn _7xnn(&mut self) {
        *self.get_mut_register_vx() += self.get_nn();
    }

    /// 将VX的值设置为为VY的值
    /// Vx = Vy
    fn _8xy0(&mut self) {
        *self.get_mut_register_vx() = self.get_register_vy();
    }

    /// 将VX的值设置为VX或VY的值（位或运算）
    /// Vx |= Vy
    fn _8xy1(&mut self) {
        *self.get_mut_register_vx() |= self.get_register_vy();
    }

    /// 将VX的值设置为VX和VY的值（位与运算）
    /// Vx &= Vy
    fn _8xy2(&mut self) {
        *self.get_mut_register_vx() &= self.get_register_vy();
    }

    ///将VX的值设置为VX和VY的值（位异或运算）
    /// Vx ^= Vy
    fn _8xy3(&mut self) {
        *self.get_mut_register_vx() ^= self.get_register_vy();
    }

    /// 将VX的值加上VY的值。
    /// 当有进位时，VF设置为1，没有设置为0。
    /// 因为只能存0～255之间的值（8位值），如果VX和VY之和大于255，就不能正确存入（
    /// 实际上又从0开始计数），我们使用进位标志让系统知道两个值的总和大于255。
    /// Vx += Vy
    fn _8xy4(&mut self) {
        let (result, overflow) = self
            .get_register_vx()
            .overflowing_add(self.get_register_vy());
        self.registers[0xF] = if overflow { 1 } else { 0 };
        *self.get_mut_register_vx() = result;
    }

    /// 设置VX为VX-VY。当有借位时VF设置为0，没有设置为1。
    /// Vx -= Vy
    fn _8xy5(&mut self) {
        let (result, overflow) = self
            .get_register_vx()
            .overflowing_sub(self.get_register_vy());
        self.registers[0xF] = if overflow { 0 } else { 1 };
        *self.get_mut_register_vx() = result;
    }

    /// 将VX的最低有效位存储在VF中，然后将VX向右移动1
    /// Vx >>= 1
    fn _8xy6(&mut self) {
        self.registers[0xF] = self.get_register_vx() & 0x1;
        *self.get_mut_register_vx() >>= 1;
    }

    /// 设置VX为VY - VX。有借位时VF设为0，没有借位时VF设为1。
    /// Vx = Vy - Vx
    fn _8xy7(&mut self) {
        let (result, overflow) = self
            .get_register_vy()
            .overflowing_sub(self.get_register_vx());
        self.registers[0xF] = if overflow { 0 } else { 1 };
        *self.get_mut_register_vx() = result;
    }

    /// 将VX的最高有效位存储在VF中，然后将VX向左移动1
    /// Vx <<= 1
    fn _8xye(&mut self) {
        self.registers[0xF] = self.get_register_vx() & 0x80;
        *self.get_mut_register_vx() <<= 1;
    }

    /// 如果VX的值不等于VY，则跳过下一条指令（通常下一条指令是跳过一个代码块）
    /// if (Vx != Vy)
    fn _9xy0(&mut self) {
        if self.get_register_vx() != self.get_register_vy() {
            self.skip_next_instruction();
        }
    }

    /// 设置索引为地址NNN
    /// I = NNN
    fn _annn(&mut self) {
        // 执行操作码
        self.index_register = self.get_nnn();
    }

    /// 跳转到V0 + 地址NNN
    /// PC = V0 + NNN
    fn _bnnn(&mut self) {
        self.program_counter = self.registers[0] as u16 + self.get_nnn();
    }

    /// 将VX设置为对一个随机数(通常为0到255)和NN进行逐位和操作的结果。
    /// Vx = rand() & NN
    fn _cxnn(&mut self) {
        *self.get_mut_register_vx() = rand::random::<u8>() & self.get_nn();
    }

    /// 绘制一个坐标(VX, VY)的精灵，其宽度为8像素，高度为N像素。
    /// 每一行8个像素被读取为位编码，从内存位置I开始，I值在执行此指令后不会改变。
    /// 如上所述，如果精灵绘制时任何屏幕像素从设置翻转到未设置，则VF设置为1，
    /// 如果没有发生这种情况，则VF设置为0。
    /// draw(Vx, Vy, N)
    fn _dxyn(&mut self) {
        let vx = self.get_register_vx() as u16;
        let vy = self.get_register_vy() as u16;
        self.registers[0xF] = 0; // 复位寄存器

        let sprite = &self.memory
            [self.index_register as usize..(self.index_register + self.get_n() as u16) as usize];

        for j in 0..sprite.len() {
            for i in 0..8 {
                let y = (vy as usize + j) % SCREEN_HEIGHT;
                let x = (vx as usize + i) % SCREEN_WIDTH;

                if (sprite[j] & (0x80 >> i)) != 0x00 {
                    if self.gfx[y][x] == 0x01 {
                        self.registers[0xF] = 1;
                    }
                    self.gfx[y][x] ^= 0x01;
                }
            }
        }
    }

    /// 如果按下存储在VX中的键，则跳过下一条指令(通常下一条指令是跳过一个代码块的跳转)。
    /// if (key() == Vx)
    fn _ex9e(&mut self) {
        if self.keypad[self.get_register_vx() as usize] {
            self.skip_next_instruction();
        } else {
            self.program_counter += 2;
        }
    }

    /// 如果没有按下存储在VX中的键，则跳过下一条指令(通常下一条指令是跳过一个代码块的跳转)。
    /// if (key() != Vx)
    fn _exa1(&mut self) {
        if !self.keypad[self.get_register_vx() as usize] {
            self.skip_next_instruction();
        } else {
            self.program_counter += 2;
        }
    }

    /// 设置VX的值为延迟定时器的值。
    /// Vx = get_delay()
    fn _fx07(&mut self) {
        *self.get_mut_register_vx() = self.delay_timer;
    }

    /// 等待一个按键，然后存储到VX（阻塞操作，所有指令停止，直到下一个按键事件）。
    /// Vx = get_key()
    fn _fx0a(&mut self) {
        self.program_counter -= 2;
        // TODO
        if self.keypad[self.get_register_vx() as usize] {
            *self.get_mut_register_vx() = self.get_register_vx();
            self.program_counter += 2;
        }
    }

    /// 将delay_timer的值设置为VX
    /// delay_timer(Vx)
    fn _fx15(&mut self) {
        self.delay_timer = self.get_register_vx();
    }

    /// 将sound_timer的值设置为VX
    /// sound_timer(vx)
    fn _fx18(&mut self) {
        self.sound_timer = self.get_register_vx();
    }

    /// 添加VX到索引寄存器，VF不受影响。
    /// I += vx
    fn _fx1e(&mut self) {
        self.index_register += self.get_register_vx() as u16;
    }

    /// 将索引寄存器设置为VX中角色的精灵位置。字符0-F(十六进制)由4x5字体表示。
    /// I = sprite_addr[Vx]
    fn _fx29(&mut self) {
        self.index_register = self.get_register_vx() as u16 * 5;
    }

    /// 将VX的二进制编码的十六进制表示形式存储在地址i、i+1、i+2
    /// set_BCD(Vx)
    /// *(I+0) = BCD(3);
    /// *(I+1) = BCD(2);
    /// *(I+2) = BCD(1);
    fn _fx33(&mut self) {
        let vx = self.get_register_vx();
        self.memory[self.index_register as usize] = vx / 100;
        self.memory[self.index_register as usize + 1] = (vx / 10) % 10;
        // self.memory[self.index_register as usize + 2] = (vx % 100) % 10; // ?
        self.memory[self.index_register as usize + 2] = vx % 10; // ?
    }

    /// 从V0到VX(包括VX)存储在内存中，从地址I开始。每写入一个值，从I的偏移量增加1，但I本身不被修改。
    /// reg_dump(Vx, &I)
    fn _fx55(&mut self) {
        for i in 0..=self.opcode.second as usize {
            self.memory[self.index_register as usize + i] = self.registers[i];
        }
    }

    /// 从V0到VX(包括VX)用内存中的值填充，从地址I开始。每读取一个值，从I的偏移量增加1，但I本身不被修改。
    /// reg_load(Vx, &I)
    fn _fx65(&mut self) {
        for i in 0..=self.opcode.second as usize {
            self.registers[i] = self.memory[self.index_register as usize + i]
        }
    }
}
