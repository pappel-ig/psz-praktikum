import p5 from "p5";
import mqtt from "mqtt";

// Composite Structure

interface Stage {
    x: number;
    y: number;

    draw(sk: p5): void;
    update(): void;
}

enum Floor {
    GROUND = "Ground",
    FIRST = "First",
    SECOND = "Second",
    THIRD = "Third",
}

// Action System for MQTT-Messages form Backend
interface Action {
    type: string;
    payload?: any;
}

class MainStage implements Stage {
    x: number = 0;
    y: number = 0;
    elevators: ElevatorStage[] = [];
    floors: FloorStage[] = [];
    actions: Action[] = [];

    FLOOR_PIXEL: Record<Floor, number> = {
        [Floor.GROUND]: 700,
        [Floor.FIRST]: 500,
        [Floor.SECOND]: 300,
        [Floor.THIRD]: 100,
    };

    draw(sk: p5): void {
        let x = 50;
        for (let e of this.elevators) {
            e.x = x;
            e.draw(sk);
            x += 300;
        }

        let y = 200;
        for (let f of [...this.floors].reverse()) {
            f.y = y;
            f.draw(sk);
            y += 200;
        }
    }

    update(): void {
        while (this.actions.length > 0) {
            const action = this.actions.shift()!;
            this.handleAction(action);
        }

        for (const e of this.elevators) e.update();
        for (const f of this.floors) f.update();
    }

    private handleAction(action: Action) {
        console.log("New Action to unwind: ", action)
        switch(action.type) {
            case "ELEVATOR_POSITION_UPDATE":
                let find = this.elevators.find(e => e.name() === action.payload.id);
                console.log(find)
                find?.setTarget(action.payload.targetY);
        }
    }

    action(action: Action) {
        this.actions.push(action);
    }

    registerElevator(stage: ElevatorStage) {
        this.elevators.push(stage);
    }

    registerFloor(stage: FloorStage) {
        this.floors.push(stage);
    }
}

class ElevatorStage implements Stage {
    x: number = 0;
    y: number = 0;
    lift: LiftStage;

    _width: number = 100;
    _height: number = 800;
    _name: string;

    constructor(name: string) {
        this._name = name;
        this.lift = new LiftStage();
    }

    draw(sk: p5): void {
        // Schacht
        sk.fill(235)
        sk.rect(this.x, 50, this._width, this._height);

        // Text
        sk.fill(10);
        sk.textSize(24);
        sk.textAlign(sk.CENTER);
        sk.text(this._name, this.x + (this._width / 2), 35);

        // Lift
        this.lift.x = this.x + 10;
        this.lift.draw(sk);
    }

    name(): string {
        return this._name;
    }

    update(): void {
        this.lift.update();
    }

    setTarget(y: number) {
        this.lift.targetY = y;
    }
}

class LiftStage implements Stage {
    x: number = 0;
    y: number = 700;
    w: number = 80;
    h: number = 100;
    targetY: number = 700;
    speed: number = 10;

    draw(sk: p5): void {
        sk.fill(255, 179, 179);
        sk.rect(this.x, this.y, this.w, this.h, 5);
    }

    update(): void {
        if (this.y < this.targetY) {
            this.y += this.speed;
        } else if (this.y > this.targetY) {
            this.y -= this.speed;
        }
    }
}

class FloorStage implements Stage {
    x: number = 0;
    y: number = 0;

    _floor: Floor;

    constructor(floor: Floor) {
        this._floor = floor;
    }

    draw(sk: p5): void {
        // Boden
        sk.stroke(1);
        sk.strokeWeight(1);
        sk.line(10, this.y, 1200, this.y);

        // Eagen-Text
        sk.fill(0);
        sk.textSize(16);
        sk.textAlign(sk.LEFT);
        sk.text(this._floor, 1220, this.y);
    }

    update(): void {

    }

    name(): Floor {
        return this._floor;
    }
}


new p5((sk) => {

    let main = new MainStage()

    sk.setup = () => {
        sk.createCanvas(1600, 850);

        main.registerElevator(new ElevatorStage("Dorisch"));
        main.registerElevator(new ElevatorStage("Ionisch"));
        main.registerElevator(new ElevatorStage("Korinthisch"));

        main.registerFloor(new FloorStage(Floor.GROUND));
        main.registerFloor(new FloorStage(Floor.FIRST));
        main.registerFloor(new FloorStage(Floor.SECOND));
        main.registerFloor(new FloorStage(Floor.THIRD));

        let client = mqtt.connect("ws://localhost:9001");

        client.on("connect", () => {
            client.subscribe("elevator/+/position");
        });

        client.on("message", (topic, msg) => {
            const json = JSON.parse(msg.toString());

            const targetY = main.FLOOR_PIXEL[json.floor as Floor];

            main.action({
                type: "ELEVATOR_POSITION_UPDATE",
                payload: {
                    id: topic.split("/")[1],
                    targetY
                }
            });
        });
    };

    sk.draw = () => {
        sk.background(250)
        main.draw(sk)
        main.update()
    };
});