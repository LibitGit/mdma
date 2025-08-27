function newSubscription(userId, sub) {
    if (typeof userId !== 'string' || typeof sub !== 'string') {
        console.warn(`\
The usage is: \`newSubscription(userId, sub);\`

where:
    userId - discord user id string,
    sub - "STARTER" | "PLUS" | "GOLD".\
`);

        return;
    }
    const updatePipeline = {
        $set: {
            exp: {
                $dateAdd: {
                    startDate: {
                        $ifNull: ["$exp", new Date()]
                    },
                    unit: "day",
                    amount: 0, // default
                }
            },
            neon: { $ifNull: ["$neon", false] },
            animation: { $ifNull: ["$animation", false] }
        },
    };

    switch (sub) {
        case "STARTER":
            updatePipeline.$set.exp.$dateAdd.amount = 1 * 30; // 1 month
            break;
        case "PLUS":
            updatePipeline.$set.exp.$dateAdd.amount = 7 * 30; // 7 months
            updatePipeline.$set.neon = true;
            break;
        case "GOLD":
            updatePipeline.$set.exp.$dateAdd.amount = 15 * 30; // 15 months
            updatePipeline.$set.neon = true;
            updatePipeline.$set.animation = true;
            break;
        default:
            return console.warn(`Unrecognised subscription: '${sub}'`);
    };

    return db.premium.updateOne(
        { _id: userId },
        [updatePipeline],
        { upsert: true }
    );
}